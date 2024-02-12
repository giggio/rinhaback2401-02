use crate::model::*;
use anyhow::Result;
use chrono::Utc;
use may_postgres::{self, Client, Statement};
use std::sync::Arc;

pub struct PgConnectionPool {
    clients: Vec<PgConnection>,
}

impl PgConnectionPool {
    pub fn new(db_url: &'static str, size: usize) -> Result<PgConnectionPool, may_postgres::Error> {
        let client_threads = (0..size)
            .map(|_| std::thread::spawn(move || PgConnection::new(db_url)))
            .collect::<Vec<_>>();
        let mut clients = client_threads
            .into_iter()
            .map(|t| t.join().unwrap())
            .collect::<Result<Vec<PgConnection>, may_postgres::Error>>()?;
        clients.sort_by(|a, b| (a.client.id() % size).cmp(&(b.client.id() % size)));
        Ok(PgConnectionPool { clients })
    }

    pub fn get_connection(&self, id: usize) -> PgConnection {
        let len = self.clients.len();
        let connection = &self.clients[id % len];
        PgConnection {
            client: connection.client.clone(),
            statement: connection.statement.clone(),
        }
    }
}

struct PgStatement {
    criar_transacao: Statement,
    get_cliente: Statement,
    get_transacoes: Statement,
}

pub struct PgConnection {
    client: Client,
    statement: Arc<PgStatement>,
}

impl PgConnection {
    fn new(db_url: &str) -> Result<Self, may_postgres::Error> {
        let client = may_postgres::connect(db_url)?;

        let criar_transacao = client.prepare("SELECT criartransacao($1, $2, $3)")?;
        let get_cliente = client.prepare("SELECT saldo, limite FROM cliente WHERE id = $1")?;
        let get_transacoes = client.prepare("SELECT valor, descricao, realizadaem FROM transacao WHERE idcliente = $1 ORDER BY id DESC LIMIT 10")?;

        let statement = Arc::new(PgStatement {
            criar_transacao,
            get_cliente,
            get_transacoes,
        });

        Ok(PgConnection { client, statement })
    }

    pub fn criar_transacao(
        &self,
        cliente_id: i32,
        transacao: Transacao,
    ) -> Result<CreateTransacaoResult, may_postgres::Error> {
        let mut q = self.client.query_raw(
            &self.statement.criar_transacao,
            &[&cliente_id, &transacao.valor, &transacao.descricao],
        )?;
        match q.next().transpose()? {
            Some(row) => {
                let create_transacao_result: CreateTransacaoResult = row.get(0);
                Ok(create_transacao_result)
            }
            None => unreachable!(
                "cliente_id={cliente_id}, transacao_valor={}, transacao_descricao={}",
                transacao.valor, transacao.descricao
            ),
        }
    }

    pub fn get_extrato(&self, cliente_id: i32) -> Result<Option<Extrato>, may_postgres::Error> {
        let cliente_opt = self.get_cliente(cliente_id)?;
        match cliente_opt {
            None => Ok(None),
            Some(cliente) => {
                let transacoes = self.get_transacoes(cliente_id)?;
                let extrato = Extrato {
                    saldo: cliente,
                    ultimas_transacoes: transacoes,
                };
                Ok(Some(extrato))
            }
        }
    }

    fn get_cliente(&self, cliente_id: i32) -> Result<Option<Saldo>, may_postgres::Error> {
        let mut q = self
            .client
            .query_raw(&self.statement.get_cliente, &[&cliente_id])?;
        let option_result_row = q.next();
        let result_option_row = option_result_row.transpose();
        let option_row = result_option_row?;
        Ok(option_row.map(|row| Saldo {
            total: row.get(0),
            limite: row.get(1),
            data_extrato: Utc::now(),
        }))
    }

    fn get_transacoes(
        &self,
        cliente_id: i32,
    ) -> Result<Vec<TransacaoComData>, may_postgres::Error> {
        let rows = self
            .client
            .query_raw(&self.statement.get_transacoes, &[&cliente_id])?;
        let all_rows = Vec::from_iter(rows.map(|r| r.unwrap()));
        let mut transacoes = Vec::with_capacity(all_rows.len());
        transacoes.extend(all_rows.into_iter().map(|r| {
            let valor: i32 = r.get(0);
            TransacaoComData {
                valor: valor.abs(),
                tipo: if valor > 0 { 'c' } else { 'd' },
                descricao: r.get(1),
                realizadaem: r.get(2),
            }
        }));
        Ok(transacoes)
    }
}

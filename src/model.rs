use chrono::{DateTime, Utc};
use may_postgres::types::FromSql;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Transacao {
    pub valor: i32,
    pub tipo: char,
    pub descricao: String,
}

#[derive(FromSql)]
#[postgres(name = "criartransacao_result")]
pub struct CreateTransacaoResult {
    pub result: i32,
    pub saldo: i32,
    pub limite: i32,
}

#[derive(Serialize)]
pub struct Transacoes {
    pub limite: i32,
    pub saldo: i32,
}

#[derive(Serialize)]
pub struct Extrato {
    pub saldo: Saldo,
    pub ultimas_transacoes: Vec<TransacaoComData>,
}

#[derive(Serialize)]
pub struct Saldo {
    pub total: i32,
    pub data_extrato: DateTime<Utc>,
    pub limite: i32,
}

#[derive(Serialize)]
pub struct TransacaoComData {
    pub valor: i32,
    pub tipo: char,
    pub descricao: String,
    pub realizadaem: DateTime<Utc>,
}

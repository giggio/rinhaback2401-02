// #[global_allocator]
// static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
use lazy_static::lazy_static;
use may_minihttp::{HttpService, HttpServiceFactory, Request, Response};
use regex::Regex;
use std::process::exit;
use std::{fmt::Write, io};
mod model;
use crate::model::*;
mod db;
use crate::db::*;

struct Rinha {
    db: PgConnection,
}

impl HttpService for Rinha {
    fn call(&mut self, req: Request, rsp: &mut Response) -> io::Result<()> {
        lazy_static! {
            static ref MATCH_CRIAR: Regex = Regex::new(r"\/clientes\/(\d+)\/transacoes.*").unwrap();
            static ref MATCH_EXTRATO: Regex = Regex::new(r"\/clientes\/(\d+)\/extrato.*").unwrap();
        }
        match req.method() {
            "GET" => {
                let p = req.path();
                if let Some(captures) = MATCH_EXTRATO.captures(p) {
                    let cliente_id = captures.get(1).unwrap().as_str().parse::<i32>().unwrap();
                    if let Some(mut extrato) = self
                        .db
                        .get_extrato(cliente_id)
                        .map_err(map_postgress_err_to_io_error)?
                    {
                        extrato.saldo.limite *= -1;
                        rsp.header("Content-Type: application/json");
                        rsp.body_mut()
                            .write_str(&serde_json::to_string(&extrato)?)
                            .map_err(|e| {
                                println!("Error writing response body: {:?}", e);
                                io::Error::new(
                                    io::ErrorKind::Other,
                                    format!("Error writing response body: {:?}", e),
                                )
                            })?;
                    } else {
                        rsp.status_code(404, "Not Found");
                    }
                } else if cfg!(debug_assertions) && p == "/healthz" {
                    rsp.header("Content-Type: text/plain").body("healthy");
                } else {
                    rsp.status_code(404, "Not Found");
                }
            }
            "POST" => {
                if let Some(captures) = MATCH_CRIAR.captures(req.path()) {
                    let cliente_id = captures.get(1).unwrap().as_str().parse::<i32>().unwrap();
                    if let Ok(mut transacao) = serde_json::from_slice::<Transacao>(req.body()) {
                        if transacao.descricao.is_empty()
                            || transacao.descricao.chars().count() > 10
                            || transacao.tipo != 'd' && transacao.tipo != 'c'
                        {
                            rsp.status_code(422, "Unprocessable Entity");
                        } else {
                            if transacao.tipo == 'd' {
                                transacao.valor *= -1;
                            }
                            let create_transacao_result = self
                                .db
                                .criar_transacao(cliente_id, transacao)
                                .map_err(map_postgress_err_to_io_error)?;
                            match create_transacao_result.result {
                                -1 => {
                                    rsp.status_code(404, "Not Found");
                                }
                                -2 => {
                                    rsp.status_code(422, "Unprocessable Entity");
                                }
                                0 => {
                                    let transacoes = Transacoes {
                                        limite: -create_transacao_result.limite,
                                        saldo: create_transacao_result.saldo,
                                    };
                                    rsp.header("Content-Type: application/json");
                                    rsp.body_mut()
                                        .write_str(&serde_json::to_string(&transacoes)?)
                                        .map_err(|e| {
                                            println!("Error writing response body: {:?}", e);
                                            io::Error::new(
                                                io::ErrorKind::Other,
                                                format!("Error writing response body: {:?}", e),
                                            )
                                        })?;
                                }
                                other => {
                                    println!(
                                        "Unexpected result when calling criartransacao: {other}"
                                    );
                                    return Err(io::Error::new(
                                        io::ErrorKind::Other,
                                        format!("Unexpected result when calling criartransacao: {other}"),
                                    ));
                                }
                            }
                        }
                    } else {
                        rsp.status_code(422, "Unprocessable Entity");
                    }
                } else {
                    rsp.status_code(404, "Not Found");
                }
            }
            _ => {
                rsp.status_code(405, "Method Not Allowed");
            }
        }
        Ok(())
    }
}

fn map_postgress_err_to_io_error(e: may_postgres::Error) -> io::Error {
    println!("Error calling database: {:?}", e);
    io::Error::new(io::ErrorKind::Other, format!("{:?}", e))
}

struct HttpServer {
    db_pool: PgConnectionPool,
}

impl HttpServiceFactory for HttpServer {
    type Service = Rinha;

    fn new_service(&self, id: usize) -> Self::Service {
        let db = self.db_pool.get_connection(id);
        Rinha { db }
    }
}

fn main() {
    let port: usize = std::env::var("PORT")
        .unwrap_or("9999".to_owned())
        .parse()
        .unwrap_or(9999);
    println!("Starting http server: 0.0.0.0:{port}...");
    may::config()
        .set_pool_capacity(1000)
        .set_stack_size(0x10000);
    lazy_static! {
        static ref CONNECTION_STRING: String = std::env::var("CONNECTION_STRING")
            .unwrap_or("postgres://rinha:rinha@localhost:5432/rinha".to_owned());
    }
    let parallelism: usize = std::env::var("PARALLELISM")
        .unwrap_or("35".to_owned())
        .parse()
        .unwrap_or(35);
    println!(
        "Connecting to database with connection string '{}', using {} parallalelism...",
        *CONNECTION_STRING, parallelism
    );
    let db_pool = match PgConnectionPool::new(&CONNECTION_STRING, parallelism) {
        Ok(db_pool) => db_pool,
        Err(e) => {
            println!("Erro ao criar pool de conexões: {:?}", e);
            exit(1);
        }
    };
    println!("Connected to database.");
    let server = HttpServer { db_pool };
    let server = match server.start(format!("0.0.0.0:{port}")) {
        Ok(server) => server,
        Err(e) => {
            println!("Error starting server: {:?}", e);
            exit(1);
        }
    };
    println!("Server started.");
    match server.join() {
        Ok(_) => {
            println!("Server stopped.");
        }
        Err(e) => {
            println!("Erro waiting for server: {:?}", e);
            exit(1);
        }
    }
}

use crate::server::ServerHandler;
use crate::transaction::Transaction;
use actix_web::{get, post, web, HttpResponse, Responder};
use std::sync::Arc;

/// Actix Web handler for posting new transactions
#[post("/transaction")]
async fn post_transaction(
    handler: web::Data<Arc<ServerHandler>>,
    transaction: web::Json<Transaction>,
) -> impl Responder {
    let success = handler
        .handle_new_transaction(transaction.into_inner())
        .await;
    if success {
        HttpResponse::Ok().body("Transaction received and validated.")
    } else {
        HttpResponse::BadRequest().body("Invalid transaction.")
    }
}

#[get("/health")]
async fn health_check() -> impl Responder {
    HttpResponse::Ok().body("OK!")
}

#[get("/create-wallet")]
async fn create_wallet() -> impl Responder {
    HttpResponse::Ok()
}

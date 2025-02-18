use crate::server::ServerHandler;
use crate::server_info;
use crate::transaction::{SignTransactionRequest, SubmitTransactionRequest};
use crate::wallet::Wallet;
use actix_web::{get, post, web, HttpResponse, Responder};
use std::sync::Arc;

/// Actix Web handler for posting new transactions
#[post("/transaction/submit")]
pub async fn submit_transaction(
    handler: web::Data<Arc<ServerHandler>>,
    transaction_request: web::Json<SubmitTransactionRequest>,
) -> impl Responder {
    let tx_request = transaction_request.into_inner();
    let tx = tx_request.transaction;
    let public_key_hex = tx_request.public_key_hex;

    let public_key = match Wallet::public_key_from_hex_string(public_key_hex) {
        Ok(public_key) => public_key,
        Err(_) => return HttpResponse::BadRequest().body("Invalid public key"),
    };

    if tx.verify(&public_key) {
        let server_handler = handler.into_inner();
        server_info!("New valid transaction received");
        {
            server_handler
                .transaction_pool
                .lock()
                .await
                .add_transaction(tx.clone());
            server_handler
                .broadcaster
                .lock()
                .await
                .broadcast_transaction(tx)
                .await;
        }
        HttpResponse::Ok().body("Transaction received and added to node.")
    } else {
        HttpResponse::BadRequest().body("Transaction not signed")
    }
}

#[get("/health")]
pub async fn health_check() -> impl Responder {
    HttpResponse::Ok().body("OK!")
}

// Ideally, a wallet should not be created inside a Node,
// transferring its data through the public internet,
// but considering this is a learning environment,
// we have this endpoint for making the process easier.
// Additionally, we have a cli tool for creating a wallet locally.
#[post("/create-wallet")]
pub async fn create_wallet() -> impl Responder {
    let export_wallet = Wallet::new().export_wallet();
    HttpResponse::Ok().json(export_wallet)
}

// WARNING - This method should be used for learning purposes only.
// Sharing public and private key, inside requests is a totally risky.
// Ideally, you should sign your transaction locally, and submit it through the node.
// You can use this struct for debugging purposes only

#[post("/transaction/sign-and-submit")]
pub async fn sign_and_submit_transaction(
    handler: web::Data<Arc<ServerHandler>>,
    sign_transaction_request: web::Json<SignTransactionRequest>,
) -> impl Responder {
    let request = sign_transaction_request.into_inner();
    let wallet = match Wallet::from_hex_string(request.public_key_hex, request.private_key_hex) {
        Ok(wallet) => wallet,
        Err(_) => return HttpResponse::BadRequest().body("Invalid wallet information"),
    };

    let mut transaction = request.transaction;
    transaction.sign(&wallet);

    let server_handler = handler.into_inner();
    server_handler
        .transaction_pool
        .lock()
        .await
        .add_transaction(transaction.clone());
    server_handler
        .broadcaster
        .lock()
        .await
        .broadcast_transaction(transaction)
        .await;
    server_info!("New valid transaction received");

    HttpResponse::Ok().body("Transaction received, signed and submitted.")
}

// WARNING - This method should be used for learning purposes only.
// Sharing public and private key, inside requests is a totally risky.
// Ideally, you should sign your transaction locally, and submit it through the node.
// You can use this struct for debugging purposes only
#[post("/transaction/sign")]
pub async fn sign_transaction(
    sign_transaction_request: web::Json<SignTransactionRequest>,
) -> impl Responder {
    let request = sign_transaction_request.into_inner();
    let wallet = match Wallet::from_hex_string(request.public_key_hex, request.private_key_hex) {
        Ok(wallet) => wallet,
        Err(_) => return HttpResponse::BadRequest().body("Invalid wallet information"),
    };
    let mut transaction = request.transaction;
    transaction.sign(&wallet);

    HttpResponse::Ok().json(transaction)
}


#[get("/transaction/{hash}")]
pub async fn get_transaction_by_hash(
    handler: web::Data<Arc<ServerHandler>>, // Assuming `ServerHandler` provides methods for fetching transactions
    path: web::Path<String>,               // The transaction hash passed as a URL parameter
) -> impl Responder {
    let hash = path.into_inner(); // Extract the transaction hash from the path
    if hash.is_empty() {
        return HttpResponse::BadRequest().body("Invalid transaction hash")
    };
    let server_handler = handler.into_inner();
    
    let result =   server_handler.database.lock().await.get_transaction(hash.as_str());
    
    match result {
        Ok(option) => {
            match option {
                None => HttpResponse::NotFound().into(),
                Some(tx) => HttpResponse::Ok().json(tx)
            }
        }
        Err(err) => {
            HttpResponse::InternalServerError().body(err.to_string())
        }
    }
}

#[get("/transaction/wallet/{address}")]
pub async fn get_transactions_by_wallet(
    handler: web::Data<Arc<ServerHandler>>, 
    path: web::Path<String>,              
) -> impl Responder {
    let address = path.into_inner(); // Extract the transaction hash from the path
    if address.is_empty() {
        return HttpResponse::BadRequest().body("Invalid wallet address")
    };
    let server_handler = handler.into_inner();

    let result =   server_handler.database.lock().await.get_transactions_by_wallet(address.as_str());

    match result {
        Ok(transactions) => HttpResponse::Ok().json(transactions),
        Err(err) => {
            HttpResponse::InternalServerError().body(err.to_string())
        }
    }
}

#[get("/block/{hash}")]
pub async fn get_block_by_hash(
    handler: web::Data<Arc<ServerHandler>>, 
    path: web::Path<String>,               
) -> impl Responder {
    let block_hash = path.into_inner();
    if block_hash.is_empty() {
        return HttpResponse::BadRequest().body("Invalid block hash")
    };

    let server_handler = handler.into_inner();
    
    let result = server_handler.database.lock().await.get_block(block_hash.as_str());

    match result {
        Some(block) => HttpResponse::Ok().json(block),
        None => {
            HttpResponse::NotFound().body("The block was not found")
        }
    }
}


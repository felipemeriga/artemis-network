use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use crate::blockchain::Blockchain;
use serde_json::to_string;
use tokio::sync::Mutex;

pub async fn handle_connection(mut socket: TcpStream, blockchain: Arc<Mutex<Blockchain>>) {
    let mut buffer = [0; 1024]; // Buffer to read incoming data

    loop {
        match socket.read(&mut buffer).await {
            Ok(0) => {
                // Connection was closed
                break;
            }
            Ok(n) => {
                let data = String::from_utf8_lossy(&buffer[..n]);
                println!("Received request: {}", data);

                // Here you could add logic to handle different types of requests.
                // For example, you could request a block or send the entire blockchain.

                // Example: If the request is "get_blockchain", send the blockchain as a response.
                if data.trim() == "get_blockchain" {
                    let blockchain_json = to_string(&blockchain.lock().await.get_chain()).unwrap();
                    println!("sending blockchain: {}", blockchain_json);
                    if let Err(e) = socket.write_all(blockchain_json.as_bytes()).await {
                        eprintln!("Failed to send blockchain: {}", e);
                    }
                } else {
                    // Handle other types of requests here
                    let response = "Unknown command".as_bytes();
                    if let Err(e) = socket.write_all(response).await {
                        eprintln!("Failed to send response: {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("Error reading from socket: {}", e);
                break;
            }
        }
    }
}

pub async fn run_server() {
    let listener = TcpListener::bind("127.0.0.1:8080")
        .await
        .unwrap();

    let blockchain = Arc::new(Mutex::new(Blockchain::new()));

    println!("Server listening on 127.0.0.1:8080");

    loop {
        let (socket, _) = listener.accept().await.unwrap();
        let blockchain_clone = Arc::clone(&blockchain);
        tokio::spawn(handle_connection(socket, blockchain_clone));
    }
}

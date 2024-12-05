use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener};
use crate::blockchain::Blockchain;
use tokio::sync::Mutex;


pub async fn run_server(address: String, blockchain: Arc<Mutex<Blockchain>>, peers: Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind(address.clone()).await.unwrap();
    println!("Server listening on {}", address);

    while let Ok((mut socket, _)) = listener.accept().await {
        let blockchain = blockchain.clone();
        let peers = peers.clone();

        tokio::spawn(async move {
            let mut buffer = [0; 1024];

            let n = socket.read(&mut buffer).await.unwrap();
            let request = String::from_utf8_lossy(&buffer[..n]);

            println!("Received request: {}", request.as_ref());

            if request.trim() == "get_blockchain" {
                let chain = blockchain.lock().await.get_chain();
                let response = serde_json::to_string(&chain).unwrap();
                socket.write_all(response.as_bytes()).await.unwrap();
            }

            if request.starts_with("add_peer") {
                let peer = request.trim()[9..].to_string();
                peers.lock().await.push(peer);
            }
        });
    }
}
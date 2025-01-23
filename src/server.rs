use crate::blockchain::Blockchain;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::{Mutex, RwLock};

pub struct Server {
    address: String,
    blockchain: Arc<RwLock<Blockchain>>,
    peers: Arc<Mutex<Vec<String>>>,
}

impl Server {
    pub fn new(
        blockchain: Arc<RwLock<Blockchain>>,
        address: String,
        peers: Arc<Mutex<Vec<String>>>,
    ) -> Self {
        Self {
            address,
            blockchain,
            peers,
        }
    }

    pub async fn run_server(&mut self) {
        let listener = TcpListener::bind(self.address.clone()).await.unwrap();
        println!("Server listening on {}", self.address);

        while let Ok((mut socket, _)) = listener.accept().await {
            let blockchain = self.blockchain.clone();
            let peers = self.peers.clone();

            tokio::spawn(async move {
                let mut buffer = [0; 1024];

                let n = socket.read(&mut buffer).await.unwrap();
                let request = String::from_utf8_lossy(&buffer[..n]);

                println!("Received request: {}", request.as_ref());

                if request.trim() == "get_blockchain" {
                    let chain = blockchain.read().await.get_chain();
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
}

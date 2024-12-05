use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;
use crate::blockchain::Blockchain;
use crate::block::Block;


pub async fn sync_with_peers(
    blockchain: Arc<Mutex<Blockchain>>,
    peers: Arc<Mutex<Vec<String>>>,
) {
    loop {
        let peers = peers.lock().await.clone();
        for peer in peers {
            if let Ok(mut stream) = TcpStream::connect(&peer).await {
                let request = "get_blockchain\n";
                if let Err(_) = stream.write_all(request.as_bytes()).await {
                    continue;
                }

                let mut buffer = [0; 1024];
                if let Ok(n) = stream.read(&mut buffer).await {
                    let data = String::from_utf8_lossy(&buffer[..n]);
                    if let Ok(peer_chain) = serde_json::from_str::<Vec<Block>>(&data) {
                        println!("Received a new chain for replacing the actual one");
                        let mut local_chain = blockchain.lock().await;
                        // Perform consensus and merge chains
                        if peer_chain.len() > local_chain.get_chain().len() {
                            local_chain.replace_chain(peer_chain);
                        }
                    }
                }
            }
        }

        // Sleep for some time before the next sync
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    }
}


use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde_json::from_str;
use crate::blockchain::Blockchain;
use crate::block::Block;

pub async fn send_request() {
    // Connect to the server
    let mut stream = TcpStream::connect("127.0.0.1:8080")
        .await
        .unwrap();

    // Send a request to the server (e.g., "get_blockchain")
    let request = "get_blockchain\n";
    if let Err(e) = stream.write_all(request.as_bytes()).await {
        eprintln!("Failed to send request: {}", e);
        return;
    }

    // Read the response from the server
    let mut buffer = [0; 1024];
    let n = stream.read(&mut buffer).await.unwrap();
    let data = String::from_utf8_lossy(&buffer[..n]);

    // println!("received blockchain: {}", &data);
    // Deserialize the response into a Blockchain object (if the response is JSON)
    let chain: Vec<Block> = from_str(&data).unwrap();
    println!("Received Blockchain: {:?}", chain);
}

pub async fn run_client() {
    send_request().await;
}

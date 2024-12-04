use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub async fn start_node(port: u16) -> tokio::io::Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", port)).await?;
    println!("Node listening on port {port}");

    loop {
        let (mut socket, _) = listener.accept().await?;
        tokio::spawn(async move {
            let mut buffer = [0; 1024];
            if let Ok(n) = socket.read(&mut buffer).await {
                println!("Received: {}", String::from_utf8_lossy(&buffer[..n]));
                socket.write_all(b"Message received").await.unwrap();
            }
        });
    }
}

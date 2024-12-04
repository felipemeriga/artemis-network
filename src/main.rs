mod block;
mod blockchain;
mod network;
mod wallet;
mod consensus;
mod server;
mod client;
mod tests;


#[tokio::main]
async fn main() {
    let server = tokio::spawn(async {
        server::run_server().await;
    });

    let client = tokio::spawn(async {
        client::run_client().await;
    });

    // Wait for both client and server to finish
    let _ = tokio::try_join!(server, client);
}

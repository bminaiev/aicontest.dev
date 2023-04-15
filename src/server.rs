use clap::Parser;
use tokio::net::TcpListener;

use crate::{connction::Connection, PORT};
use anyhow::Result;

#[derive(Parser)]
pub struct Args {
    #[clap(long)]
    port: Option<u16>,
}

async fn handle_connection(mut conn: Connection) -> Result<()> {
    conn.write("LOGIN").await?;
    let login = conn.read_token().await?;
    log::info!("Got login: {login}");
    loop {
        let xx = conn.read_token().await?;
        log::info!("Received {xx} from {:?}", conn.addr);
        if xx == "EXIT" {
            break;
        }
    }
    Ok(())
}

#[tokio::main]
pub async fn run(args: Args) -> Result<()> {
    let port = args.port.unwrap_or(PORT);

    let listener = TcpListener::bind(format!("0.0.0.0:{port}")).await.unwrap();
    log::info!("Listening to port {port}");

    loop {
        let (tcp_stream, addr) = listener.accept().await.unwrap();
        log::info!("New connection from {addr:?}");
        tokio::spawn(async move {
            match handle_connection(Connection::new(tcp_stream, addr.clone())).await {
                Ok(()) => log::info!("Connection {addr:?} closed successfully."),
                Err(e) => log::info!("Some error happened with connection to {addr:?}: {e}"),
            }
        });
    }
}

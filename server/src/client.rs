use std::{net::SocketAddr, str::FromStr};

use clap::Parser;
use tokio::net::TcpStream;

use crate::{connction::Connection, PORT};
use anyhow::Result;

#[derive(Parser)]
pub struct Args {
    #[clap(long)]
    addr: Option<String>,
}

const MY_LOGIN: &str = "qwerty787788";

#[tokio::main]
pub async fn run(args: Args) -> Result<()> {
    let addr = args.addr.unwrap_or(format!("127.0.0.1:{}", PORT));
    log::info!("Trying to connect to {addr}");
    let stream = TcpStream::connect(addr.clone()).await?;
    let mut conn = Connection::new(stream, SocketAddr::from_str(&addr).unwrap());
    loop {
        conn.read_expect("LOGIN").await?;
        conn.write(MY_LOGIN).await?;
    }
}

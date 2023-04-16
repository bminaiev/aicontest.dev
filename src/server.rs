use std::sync::Arc;

use clap::Parser;
use tokio::{net::TcpListener, sync::watch};

use crate::{connction::Connection, engine, game_state::GameState, PORT};
use anyhow::Result;

#[derive(Parser)]
pub struct Args {
    #[clap(long)]
    port: Option<u16>,
}

// TODO: watch/play
async fn handle_connection(
    mut conn: Connection,
    mut rx_game_stats: watch::Receiver<Option<GameState>>,
) -> Result<()> {
    conn.write("LOGIN").await?;
    let login = conn.read_token().await?;
    log::info!("Got login: {login}");
    // TODO: better auth.
    let mut state;
    loop {
        rx_game_stats.changed().await?;
        state = rx_game_stats.borrow().clone();
        if let Some(state) = &mut state {
            state.make_player_first(&login);
            state.send_to_conn(&mut conn).await?;
        }
        // TODO: reshuffle players to make you first.

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

    let (tx_game_states, rx_game_states) = watch::channel::<Option<GameState>>(None);
    tokio::spawn(async move { engine::run(tx_game_states).await });

    // TODO: ip-based rate limiting.
    loop {
        let (tcp_stream, addr) = listener.accept().await.unwrap();
        log::info!("New connection from {addr:?}");
        let rx_game_states = rx_game_states.clone();
        tokio::spawn(async move {
            match handle_connection(
                Connection::new(tcp_stream, addr.clone()),
                rx_game_states.clone(),
            )
            .await
            {
                Ok(()) => log::info!("Connection {addr:?} closed successfully."),
                Err(e) => log::info!("Some error happened with connection to {addr:?}: {e}"),
            }
        });
    }
}

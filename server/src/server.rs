use clap::Parser;
use tokio::{
    net::TcpListener,
    sync::{mpsc, watch},
};

use crate::{
    connction::Connection, engine, game_state::GameState, player_move::PlayerMove, point::Point,
    PORT,
};
use anyhow::Result;

#[derive(Parser)]
pub struct Args {
    #[clap(long)]
    port: Option<u16>,
}

async fn handle_play(
    mut conn: Connection,
    mut rx_game_stats: watch::Receiver<Option<GameState>>,
    tx_moves: mpsc::Sender<PlayerMove>,
) -> Result<()> {
    let login = conn.read_token().await?;
    // TODO: validation
    log::info!("Got login: {login}");
    // TODO: better auth.
    let mut state;
    loop {
        rx_game_stats.changed().await?;
        state = rx_game_stats.borrow().clone();
        if let Some(state) = &mut state {
            if !state.make_player_first(&login) {
                tx_moves
                    .send(PlayerMove {
                        name: login.clone(),
                        target: Point::ZERO,
                    })
                    .await?;
                continue;
            }
            state.send_to_conn(&mut conn).await?;

            let cmd = conn.read_token().await?;
            if cmd == "GO" {
                let x: i32 = conn.read().await?;
                let y: i32 = conn.read().await?;
                tx_moves
                    .send(PlayerMove {
                        name: login.clone(),
                        target: Point { x, y },
                    })
                    .await?;
            } else if cmd == "EXIT" {
                return Ok(());
            } else {
                conn.write(format!("UNKNOWN command '{cmd}', expected 'GO' or 'EXIT'"))
                    .await?;
            }
        }
    }
}

async fn handle_watch(
    mut conn: Connection,
    mut rx_game_stats: watch::Receiver<Option<GameState>>,
) -> Result<()> {
    let mut state;
    loop {
        rx_game_stats.changed().await?;
        state = rx_game_stats.borrow().clone();
        if let Some(state) = &mut state {
            state.send_to_conn(&mut conn).await?;
        }
    }
}

// TODO: watch/play
async fn handle_connection(
    mut conn: Connection,
    rx_game_stats: watch::Receiver<Option<GameState>>,
    tx_moves: mpsc::Sender<PlayerMove>,
) -> Result<()> {
    conn.write("HELLO").await?;
    loop {
        let cmd_type = conn.read_token().await?;
        if cmd_type == "WATCH" {
            handle_watch(conn, rx_game_stats).await?;
            break;
        }
        if cmd_type == "PLAY" {
            handle_play(conn, rx_game_stats, tx_moves).await?;
            break;
        }
        conn.write(format!("Expected 'WATCH' or 'PLAY', got '{}'", cmd_type))
            .await?;
    }
    Ok(())
}

#[tokio::main]
pub async fn run(args: Args) -> Result<()> {
    let port = args.port.unwrap_or(PORT);

    let listener = TcpListener::bind(format!("0.0.0.0:{port}")).await.unwrap();
    log::info!("Listening to port {port}");

    let (tx_game_states, rx_game_states) = watch::channel::<Option<GameState>>(None);
    let (tx_moves, rx_moves) = mpsc::channel::<PlayerMove>(1024);
    tokio::spawn(async move { engine::run(tx_game_states, rx_moves).await });

    // TODO: ip-based rate limiting.
    loop {
        let (tcp_stream, addr) = listener.accept().await.unwrap();
        log::info!("New connection from {addr:?}");
        let rx_game_states = rx_game_states.clone();
        let tx_moves = tx_moves.clone();
        tokio::spawn(async move {
            match handle_connection(
                Connection::new(tcp_stream, addr.clone()),
                rx_game_states,
                tx_moves,
            )
            .await
            {
                Ok(()) => log::info!("Connection {addr:?} closed successfully."),
                Err(e) => log::info!("Some error happened with connection to {addr:?}: {e}"),
            }
        });
    }
}

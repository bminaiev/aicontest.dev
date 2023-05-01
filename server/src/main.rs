use std::sync::Arc;

use clap::Parser;

pub mod connection;
pub mod engine;
pub mod password_manager;
pub mod top_results;

use game_common::{
    consts::MAX_LOGIN_LEN, game_state::GameState, player_move::PlayerMove, point::Point,
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{mpsc, watch},
};
use tokio_tungstenite::{accept_async, tungstenite::Message};

use crate::{connection::Connection, password_manager::PasswordManager, top_results::TopResults};
use anyhow::Result;
use futures_util::{stream::StreamExt, SinkExt};

#[derive(Parser)]
pub struct Args {
    #[clap(long)]
    tcp_port: Option<u16>,
    #[clap(long)]
    websocket_port: Option<u16>,
    #[clap(long)]
    base_dir: Option<String>,
}

fn validate_login(login: &str) -> Result<()> {
    if login.len() > MAX_LOGIN_LEN {
        return Err(anyhow::anyhow!(
            "Login is too long. MAX_LOGIN_LEN = {}",
            MAX_LOGIN_LEN
        ));
    }
    if !login.is_ascii() {
        return Err(anyhow::anyhow!("Login must be ASCII"));
    }
    Ok(())
}

async fn handle_tcp_play(
    mut conn: Connection,
    mut rx_game_stats: watch::Receiver<Option<GameState>>,
    tx_moves: mpsc::Sender<PlayerMove>,
    password_manager: Arc<PasswordManager>,
) -> Result<()> {
    let login = conn.read_token().await?;
    log::info!("Got login: {login}");
    validate_login(&login)?;
    let password = conn.read_token().await?;
    log::debug!("Got password: {password}");
    if let Err(err) = password_manager
        .check_password(&login, &password, &conn.addr.to_string())
        .await
    {
        conn.write(&format!("{:?}", err)).await?;
        return Err(err);
    }
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
            conn.write(state.to_string()).await?;

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

async fn handle_tcp_watch(
    mut conn: Connection,
    mut rx_game_states: watch::Receiver<Option<GameState>>,
) -> Result<()> {
    let mut state;
    loop {
        rx_game_states.changed().await?;
        state = rx_game_states.borrow().clone();
        if let Some(state) = &mut state {
            conn.write(state.to_string()).await?;
        }
    }
}

async fn handle_tcp_connection(
    mut conn: Connection,
    rx_game_states: watch::Receiver<Option<GameState>>,
    tx_moves: mpsc::Sender<PlayerMove>,
    password_manager: Arc<PasswordManager>,
) -> Result<()> {
    conn.write("HELLO").await?;
    loop {
        let cmd_type = conn.read_token().await?;
        if cmd_type == "WATCH" {
            handle_tcp_watch(conn, rx_game_states).await?;
            break;
        }
        if cmd_type == "PLAY" {
            handle_tcp_play(conn, rx_game_states, tx_moves, password_manager).await?;
            break;
        }
        conn.write(format!("Expected 'WATCH' or 'PLAY', got '{}'", cmd_type))
            .await?;
    }
    Ok(())
}

async fn run_tcp_server(
    port: u16,
    rx_game_states: watch::Receiver<Option<GameState>>,
    tx_moves: mpsc::Sender<PlayerMove>,
    password_manager: Arc<PasswordManager>,
) {
    let listener = TcpListener::bind(format!("0.0.0.0:{port}")).await.unwrap();
    log::info!("Listening to port {port}");

    // TODO: ip-based rate limiting.
    loop {
        let (tcp_stream, addr) = listener.accept().await.unwrap();
        log::info!("New connection from {addr:?}");
        let rx_game_states = rx_game_states.clone();
        let tx_moves = tx_moves.clone();
        let password_manager = password_manager.clone();
        tokio::spawn(async move {
            match handle_tcp_connection(
                Connection::new(tcp_stream, addr.clone()),
                rx_game_states,
                tx_moves,
                password_manager,
            )
            .await
            {
                Ok(()) => log::info!("Connection {addr:?} closed successfully."),
                Err(e) => log::info!("Some error happened with connection to {addr:?}: {e}"),
            }
        });
    }
}

async fn handle_one_web_socket_conn(
    raw_stream: TcpStream,
    mut rx_game_states: watch::Receiver<Option<GameState>>,
) -> Result<()> {
    let ws_stream = accept_async(raw_stream).await.unwrap();
    let (mut write, _read) = ws_stream.split();

    let mut state;
    loop {
        rx_game_states.changed().await?;
        state = rx_game_states.borrow().clone();
        if let Some(state) = &mut state {
            write.send(Message::Text(state.to_string())).await?;
        }
    }
}

async fn run_web_sockets_server(port: u16, rx_game_states: watch::Receiver<Option<GameState>>) {
    let listener = TcpListener::bind(format!("0.0.0.0:{port}")).await.unwrap();
    log::info!("WebSocket server listening on ws://127.0.0.1:{port}");

    while let Ok((raw_stream, addr)) = listener.accept().await {
        log::info!("New websocket connection from {addr:?}");

        let rx_game_states = rx_game_states.clone();
        tokio::spawn(async move {
            let res = handle_one_web_socket_conn(raw_stream, rx_game_states).await;
            log::info!("Websocket connection from {addr:?} closed: {res:?}");
        });
    }
}

pub const DEFAULT_TCP_PORT: u16 = 7877;
pub const DEFAULT_WEB_SOCKET_PORT: u16 = 7878;

#[tokio::main]
pub async fn main() -> Result<()> {
    env_logger::init();
    log::info!("Starting server");
    let args = Args::parse();

    let base_dir = args.base_dir.unwrap_or("data".to_owned());
    let password_manager =
        Arc::new(PasswordManager::new(format!("{base_dir}/passwords.txt")).await?);
    let games_dir = format!("{base_dir}/games");
    let mut top_results = TopResults::new(format!("{base_dir}/top_results.txt")).await?;

    let tcp_port = args.tcp_port.unwrap_or(DEFAULT_TCP_PORT);
    let web_socket_port = args.websocket_port.unwrap_or(DEFAULT_WEB_SOCKET_PORT);

    let (tx_game_states, rx_game_states) = watch::channel::<Option<GameState>>(None);
    let (tx_moves, rx_moves) = mpsc::channel::<PlayerMove>(1024);
    tokio::spawn(async move {
        engine::run(tx_game_states, rx_moves, &games_dir, &mut top_results).await
    });

    let tcp_server = tokio::spawn({
        let rx_game_states = rx_game_states.clone();
        async move { run_tcp_server(tcp_port, rx_game_states, tx_moves, password_manager).await }
    });

    let websockets_server =
        tokio::spawn(async move { run_web_sockets_server(web_socket_port, rx_game_states).await });

    tokio::try_join!(tcp_server, websockets_server)?;

    Ok(())
}

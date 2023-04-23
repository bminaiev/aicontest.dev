use clap::Parser;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{mpsc, watch},
};
use tokio_tungstenite::{accept_async, tungstenite::Message};

use crate::{
    connction::Connection, engine, game_state::GameState, player_move::PlayerMove, point::Point,
    DEFAULT_TCP_PORT, DEFAULT_WEB_SOCKET_PORT,
};
use anyhow::Result;
use futures_util::{stream::StreamExt, SinkExt};

#[derive(Parser)]
pub struct Args {
    #[clap(long)]
    tcp_port: Option<u16>,
    #[clap(long)]
    websocket_port: Option<u16>,
}

async fn handle_tcp_play(
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

async fn handle_tcp_watch(
    mut conn: Connection,
    mut rx_game_states: watch::Receiver<Option<GameState>>,
) -> Result<()> {
    let mut state;
    loop {
        rx_game_states.changed().await?;
        state = rx_game_states.borrow().clone();
        if let Some(state) = &mut state {
            state.send_to_conn(&mut conn).await?;
        }
    }
}

// TODO: watch/play
async fn handle_tcp_connection(
    mut conn: Connection,
    rx_game_states: watch::Receiver<Option<GameState>>,
    tx_moves: mpsc::Sender<PlayerMove>,
) -> Result<()> {
    conn.write("HELLO").await?;
    loop {
        let cmd_type = conn.read_token().await?;
        if cmd_type == "WATCH" {
            handle_tcp_watch(conn, rx_game_states).await?;
            break;
        }
        if cmd_type == "PLAY" {
            handle_tcp_play(conn, rx_game_states, tx_moves).await?;
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
) {
    let listener = TcpListener::bind(format!("0.0.0.0:{port}")).await.unwrap();
    log::info!("Listening to port {port}");

    // TODO: ip-based rate limiting.
    loop {
        let (tcp_stream, addr) = listener.accept().await.unwrap();
        log::info!("New connection from {addr:?}");
        let rx_game_states = rx_game_states.clone();
        let tx_moves = tx_moves.clone();
        tokio::spawn(async move {
            match handle_tcp_connection(
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

#[tokio::main]
pub async fn run(args: Args) -> Result<()> {
    let tcp_port = args.tcp_port.unwrap_or(DEFAULT_TCP_PORT);
    let web_socket_port = args.websocket_port.unwrap_or(DEFAULT_WEB_SOCKET_PORT);

    let (tx_game_states, rx_game_states) = watch::channel::<Option<GameState>>(None);
    let (tx_moves, rx_moves) = mpsc::channel::<PlayerMove>(1024);
    tokio::spawn(async move { engine::run(tx_game_states, rx_moves).await });

    let tcp_server = tokio::spawn({
        let rx_game_states = rx_game_states.clone();
        async move { run_tcp_server(tcp_port, rx_game_states, tx_moves).await }
    });

    let websockets_server =
        tokio::spawn(async move { run_web_sockets_server(web_socket_port, rx_game_states).await });

    tokio::try_join!(tcp_server, websockets_server)?;

    Ok(())
}

use std::{net::SocketAddr, str::FromStr, time::Duration};

use clap::Parser;
use game_common::{game_state::GameState, player_move::PlayerMove};
use tokio::net::TcpStream;

mod connection;

use crate::connection::Connection;
use anyhow::Result;

#[derive(Parser)]
pub struct Args {
    #[clap(long)]
    addr: Option<String>,
    #[clap(long, default_value_t = 1)]
    num_bots: usize,
}

const MY_LOGIN_PREFIX: &str = "basic-rust-";

fn best_move(game_state: &GameState) -> PlayerMove {
    let me = &game_state.players[0];
    let mut go_to = me.pos;
    for item in game_state.items.iter() {
        if go_to == me.pos || item.pos.dist2(&me.pos) < go_to.dist2(&me.pos) {
            go_to = item.pos;
        }
    }
    // TODO: why do I need to pass name?)
    PlayerMove {
        name: me.name.clone(),
        target: go_to,
    }
}

async fn try_one_game(addr: &str, login: &str, password: &str) -> Result<()> {
    log::info!("Trying to connect to {addr}");
    let stream = TcpStream::connect(addr.clone()).await?;
    let mut conn = Connection::new(stream, SocketAddr::from_str(&addr).unwrap());

    conn.read_expect("HELLO").await?;
    conn.write("PLAY").await?;
    conn.write(format!("{login} {password}")).await?;
    let mut last_seen_turn = usize::MAX;
    loop {
        let mut state = vec![];
        loop {
            let next_token: String = conn.read().await?;
            let should_end = next_token == "END_STATE";
            state.push(next_token);
            if should_end {
                break;
            }
        }
        match GameState::from_string(&state.join(" ")) {
            Ok(game_state) => {
                let turn = game_state.turn;
                if turn < last_seen_turn {
                    log::info!("New game started. Current turn: {turn}");
                }
                last_seen_turn = turn;
                let my_move = best_move(&game_state);
                conn.write(&format!("GO {} {}", my_move.target.x, my_move.target.y))
                    .await?;
            }
            Err(err) => {
                anyhow::bail!("Error while parsing state: {}", err);
            }
        }
    }
}

async fn one_client(addr: String) {
    let login = format!("{}{}", MY_LOGIN_PREFIX, rand::random::<u8>());
    let password = "very-secure-password";
    loop {
        match try_one_game(&addr, &login, &password).await {
            Ok(()) => {}
            Err(err) => {
                log::error!("Connection finished with error: {}", err);
            }
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

#[tokio::main]
pub async fn main() -> Result<()> {
    env_logger::init();
    log::info!("Starting client");
    let args = Args::parse();

    let addr = args.addr.unwrap_or(format!("127.0.0.1:7877"));
    let mut handles = vec![];
    for _ in 0..args.num_bots {
        handles.push(tokio::spawn(one_client(addr.clone())));
    }
    for handle in handles {
        handle.await?;
    }
    Ok(())
}

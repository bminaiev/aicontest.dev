use clap::{Parser, Subcommand};

pub mod client;
pub mod connction;
pub mod consts;
pub mod engine;
pub mod game_state;
pub mod player_move;
pub mod point;
pub mod server;

#[derive(Parser)]
struct Args {
    #[clap(subcommand)]
    command: Subcommands,
}

#[derive(Subcommand)]
enum Subcommands {
    Server(server::Args),
    Client(client::Args),
}

pub const DEFAULT_TCP_PORT: u16 = 7877;
pub const DEFAULT_WEB_SOCKET_PORT: u16 = 7878;

fn main() -> Result<(), anyhow::Error> {
    env_logger::init();
    match Args::parse().command {
        Subcommands::Server(args) => server::run(args),
        Subcommands::Client(args) => client::run(args),
    }
}

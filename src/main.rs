use clap::{Parser, Subcommand};

pub mod client;
pub mod connction;
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

pub const PORT: u16 = 7877;

fn main() -> Result<(), anyhow::Error> {
    env_logger::init();
    match Args::parse().command {
        Subcommands::Server(args) => server::run(args),
        Subcommands::Client(args) => client::run(args),
    }
}

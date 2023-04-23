use tokio::sync::{mpsc, watch};

use game_common::consts::TURN_WAIT_TIME;
use game_common::game_state::{self, GameState};
use game_common::player_move::PlayerMove;

pub async fn run(
    tx_game_states: watch::Sender<Option<GameState>>,
    mut rx_moves: mpsc::Receiver<PlayerMove>,
) {
    log::info!("Running games...");
    loop {
        log::info!("New game!");
        let mut state = GameState::new();
        loop {
            log::info!("TURN {}. Players: {}.", state.turn, state.players.len());
            tx_game_states.send_replace(Some(state.clone()));
            tokio::time::sleep(TURN_WAIT_TIME).await;
            // TODO: accept commands in parallel with waiting.
            while let Ok(player_move) = rx_moves.try_recv() {
                state.apply_move(player_move);
            }
            match state.next_turn() {
                game_state::NextTurn::GameState(next_state) => {
                    state = next_state;
                }
                game_state::NextTurn::FinalResults(results) => {
                    log::info!("Game finished! Results:");
                    for player in results.players.iter() {
                        log::info!("{}: {}", player.name, player.score);
                    }
                    break;
                }
            }
        }
    }
}

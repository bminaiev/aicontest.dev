use tokio::sync::watch::Sender;

use crate::{consts::TURN_WAIT_TIME, game_state::GameState};

pub async fn run(tx_game_states: Sender<Option<GameState>>) {
    log::info!("Running games...");
    loop {
        let mut state = GameState::new();
        loop {
            log::info!("TURN {}. Players: {}.", state.turn, state.players.len());
            tx_game_states.send_replace(Some(state.clone()));
            tokio::time::sleep(TURN_WAIT_TIME).await;
            // TODO: accept commands
            match state.next_turn() {
                crate::game_state::NextTurn::GameState(next_state) => {
                    state = next_state;
                }
                crate::game_state::NextTurn::FinalResults(results) => {
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

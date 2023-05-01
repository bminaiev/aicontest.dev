use std::{collections::HashSet, path};

use anyhow::Context;
use game_common::game_state::GameResults;
use tokio::{
    fs::{create_dir_all, OpenOptions},
    io::{AsyncReadExt, AsyncWriteExt},
};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct UserResult {
    score: i64,
    game_id: String,
    user: String,
}

pub struct TopResults {
    results: Vec<UserResult>,
    filename: String,
}

impl TopResults {
    pub async fn new(filename: String) -> anyhow::Result<Self> {
        create_dir_all(path::Path::new(&filename).parent().unwrap()).await?;
        let mut results = vec![];
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(filename.clone())
            .await
            .context(format!("Open top-results file: {filename}"))?;
        let mut lines = String::new();
        file.read_to_string(&mut lines).await?;
        for line in lines.lines() {
            let mut parts = line.split(' ');
            let user = parts.next().unwrap().to_owned();
            let game_id: String = parts.next().unwrap().to_owned();
            let score = parts.next().unwrap().parse()?;
            results.push(UserResult {
                score,
                game_id,
                user,
            })
        }
        Ok(Self { results, filename })
    }

    pub async fn add_results(&mut self, game_result: GameResults) -> anyhow::Result<()> {
        for player in game_result.players.iter() {
            self.results.push(UserResult {
                score: player.score,
                game_id: game_result.game_id.clone(),
                user: player.name.clone(),
            });
        }
        self.results.sort();
        self.results.reverse();

        let mut new_results = vec![];
        let mut seen_users = HashSet::new();
        for res in self.results.iter() {
            if seen_users.contains(&res.user) {
                continue;
            }
            seen_users.insert(res.user.clone());
            new_results.push(res.clone());
        }
        self.results = new_results;

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(self.filename.clone())
            .await?;
        for result in &self.results {
            file.write_all(
                format!("{} {} {}\n", result.user, result.game_id, result.score).as_bytes(),
            )
            .await?;
        }
        Ok(())
    }
}

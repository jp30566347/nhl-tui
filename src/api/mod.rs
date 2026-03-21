pub mod models;

use color_eyre::eyre::Result;
use models::*;
use std::collections::HashMap;

const BASE_URL: &str = "https://api-web.nhle.com/v1";

pub struct NhlClient {
    client: reqwest::Client,
}

impl NhlClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent(concat!("nhl-tui/", env!("CARGO_PKG_VERSION")))
                .build()
                .expect("failed to build HTTP client"),
        }
    }

    pub async fn get_scores(&self, date: &str) -> Result<ScoreResponse> {
        let url = format!("{}/score/{}", BASE_URL, date);
        Ok(self.client.get(&url).send().await?.json().await?)
    }

    pub async fn get_standings(&self) -> Result<StandingsResponse> {
        let url = format!("{}/standings/now", BASE_URL);
        Ok(self.client.get(&url).send().await?.json().await?)
    }

    pub async fn get_schedule(&self, date: &str) -> Result<ScheduleResponse> {
        let url = format!("{}/schedule/{}", BASE_URL, date);
        let resp = self.client.get(&url).send().await?;
        let text = resp.text().await?;
        serde_json::from_str(&text).map_err(|e| color_eyre::eyre::eyre!("Schedule parse error: {} | first 200 chars: {}", e, &text[..200.min(text.len())]))
    }

    pub async fn get_leaders(&self, category: &str, limit: u32) -> Result<Vec<StatLeader>> {
        let url = format!(
            "{}/skater-stats-leaders/current?categories={}&limit={}",
            BASE_URL, category, limit
        );
        let resp = self.client.get(&url).send().await?;
        let text = resp.text().await?;
        let map: HashMap<String, Vec<StatLeader>> = serde_json::from_str(&text)
            .map_err(|e| color_eyre::eyre::eyre!("Leaders parse error for '{}': {} | first 200: {}", category, e, &text[..200.min(text.len())]))?;
        Ok(map.get(category).cloned().unwrap_or_default())
    }

    pub async fn get_boxscore(&self, game_id: u64) -> Result<BoxscoreResponse> {
        let url = format!("{}/gamecenter/{}/landing", BASE_URL, game_id);
        Ok(self.client.get(&url).send().await?.json().await?)
    }
}

pub mod models;

use std::collections::HashMap;
use std::time::Duration;

use color_eyre::eyre::{Context, Result};
use models::*;

const BASE_URL: &str = "https://api-web.nhle.com/v1";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// Cheap to clone: `reqwest::Client` is internally reference counted.
#[derive(Clone)]
pub struct NhlClient {
    client: reqwest::Client,
}

impl Default for NhlClient {
    fn default() -> Self {
        Self::new()
    }
}

impl NhlClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent(concat!("nhl-tui/", env!("CARGO_PKG_VERSION")))
                .timeout(REQUEST_TIMEOUT)
                .connect_timeout(CONNECT_TIMEOUT)
                .build()
                .expect("failed to build HTTP client"),
        }
    }

    async fn get_json<T: serde::de::DeserializeOwned>(&self, url: &str, what: &str) -> Result<T> {
        let resp = self
            .client
            .get(url)
            .send()
            .await
            .with_context(|| format!("{what}: request failed"))?
            .error_for_status()
            .with_context(|| format!("{what}: bad status"))?;
        let body = resp
            .text()
            .await
            .with_context(|| format!("{what}: could not read body"))?;
        serde_json::from_str(&body)
            .with_context(|| format!("{what}: could not parse response ({})", preview(&body)))
    }

    pub async fn get_scores(&self, date: &str) -> Result<ScoreResponse> {
        self.get_json(&format!("{BASE_URL}/score/{date}"), "scores")
            .await
    }

    pub async fn get_standings(&self) -> Result<StandingsResponse> {
        self.get_json(&format!("{BASE_URL}/standings/now"), "standings")
            .await
    }

    pub async fn get_schedule(&self, date: &str) -> Result<ScheduleResponse> {
        self.get_json(&format!("{BASE_URL}/schedule/{date}"), "schedule")
            .await
    }

    /// Omitting `categories` returns every category the endpoint supports in a
    /// single response, keyed by category name.
    pub async fn get_leaders(&self, limit: u32) -> Result<HashMap<String, Vec<StatLeader>>> {
        self.get_json(
            &format!("{BASE_URL}/skater-stats-leaders/current?limit={limit}"),
            "leaders",
        )
        .await
    }

    pub async fn get_boxscore(&self, game_id: u64) -> Result<BoxscoreResponse> {
        self.get_json(
            &format!("{BASE_URL}/gamecenter/{game_id}/landing"),
            "boxscore",
        )
        .await
    }
}

/// First 200 characters of a response body, for error messages.
///
/// Truncates on a character boundary; slicing by byte index would panic on
/// multi-byte UTF-8 (player names routinely contain accents).
fn preview(body: &str) -> String {
    let mut s: String = body.chars().take(200).collect();
    if s.len() < body.len() {
        s.push('\u{2026}');
    }
    s
}

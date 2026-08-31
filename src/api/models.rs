use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct ScoreResponse {
    pub games: Vec<Game>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Game {
    pub id: u64,
    /// The API spells this `startTimeUTC`, which `rename_all = "camelCase"`
    /// would otherwise map to `startTimeUtc` and silently leave as `None`.
    #[serde(rename = "startTimeUTC")]
    pub start_time_utc: Option<String>,
    pub game_state: String,
    pub away_team: TeamScore,
    pub home_team: TeamScore,
    pub game_outcome: Option<GameOutcome>,
    pub period: Option<u32>,
    pub period_descriptor: Option<PeriodDescriptor>,
    pub clock: Option<GameClock>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TeamScore {
    pub abbrev: String,
    pub score: Option<u32>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TeamName {
    pub default: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GameOutcome {
    pub last_period_type: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GameClock {
    pub time_remaining: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PeriodDescriptor {
    pub number: u32,
    pub period_type: Option<String>,
}

impl PeriodDescriptor {
    /// "1st", "2nd", "3rd", "OT", "2OT", "SO".
    pub fn label(&self) -> String {
        match self.period_type.as_deref() {
            Some("SO") => "SO".to_string(),
            _ => match self.number {
                1 => "1st".to_string(),
                2 => "2nd".to_string(),
                3 => "3rd".to_string(),
                4 => "OT".to_string(),
                // saturating: the period number is server-supplied, and 0
                // would underflow.
                n => format!("{}OT", n.saturating_sub(3)),
            },
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct StandingsResponse {
    pub standings: Vec<Standing>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Standing {
    pub team_name: TeamName,
    pub team_abbrev: TeamAbbrev,
    pub conference_name: String,
    pub division_name: String,
    pub games_played: u32,
    pub wins: u32,
    pub losses: u32,
    pub ot_losses: u32,
    pub points: u32,
    pub goal_for: u32,
    pub goal_against: u32,
    pub goal_differential: i32,
    pub streak_code: Option<String>,
    pub streak_count: Option<u32>,
    pub division_sequence: Option<u32>,
    pub conference_sequence: Option<u32>,
    pub league_sequence: Option<u32>,
    pub point_pctg: Option<f64>,
    pub l10_wins: Option<u32>,
    pub l10_losses: Option<u32>,
    pub l10_ot_losses: Option<u32>,
    /// 0 for a team holding a top-three spot in its division; 1 and 2 are the
    /// two wild card berths; 3 and up are outside the playoff picture.
    pub wildcard_sequence: Option<u32>,
}

impl Standing {
    /// Whether the team currently holds a playoff berth: a divisional top
    /// three, or one of the conference's two wild cards.
    pub fn in_playoff_spot(&self) -> bool {
        matches!(self.wildcard_sequence, Some(0..=2))
    }

    /// Record over the last ten games, as "W-L-OTL".
    pub fn last_ten(&self) -> Option<String> {
        Some(format!(
            "{}-{}-{}",
            self.l10_wins?, self.l10_losses?, self.l10_ot_losses?
        ))
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct TeamAbbrev {
    pub default: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleResponse {
    pub game_week: Vec<GameDay>,
    /// Nearest dates on either side that actually have games, and the season
    /// boundaries. These are what make an empty week actionable.
    pub next_start_date: Option<String>,
    pub previous_start_date: Option<String>,
    pub regular_season_start_date: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GameDay {
    pub date: String,
    pub day_abbrev: String,
    pub games: Vec<ScheduleGame>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleGame {
    #[serde(rename = "startTimeUTC")]
    pub start_time_utc: String,
    pub away_team: ScheduleTeam,
    pub home_team: ScheduleTeam,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleTeam {
    pub abbrev: String,
    pub place_name: Option<TeamName>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StatLeader {
    pub first_name: Option<NameField>,
    pub last_name: Option<NameField>,
    pub position: Option<String>,
    pub team_abbrev: Option<String>,
    /// Integer for counting stats, fractional for faceoff % and time on ice.
    pub value: Option<f64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct NameField {
    pub default: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BoxscoreResponse {
    pub away_team: BoxscoreTeam,
    pub home_team: BoxscoreTeam,
    pub summary: Option<Summary>,
}

/// The `/boxscore` endpoint, which is where shots on goal live; the
/// `/landing` one above carries the scoring and penalty summaries.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GameStats {
    pub away_team: TeamStats,
    pub home_team: TeamStats,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TeamStats {
    pub sog: Option<u32>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub scoring: Option<Vec<ScoringPeriod>>,
    pub penalties: Option<Vec<PenaltyPeriod>>,
    pub three_stars: Option<Vec<ThreeStar>>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PenaltyPeriod {
    pub period_descriptor: PeriodDescriptor,
    pub penalties: Vec<Penalty>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Penalty {
    pub time_in_period: String,
    pub duration: Option<u32>,
    pub committed_by_player: Option<PlayerName>,
    pub team_abbrev: Option<NameField>,
    /// A slug such as "interference"; rendered with the underscores removed.
    pub desc_key: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PlayerName {
    pub first_name: Option<NameField>,
    pub last_name: Option<NameField>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ThreeStar {
    pub star: u32,
    pub name: Option<NameField>,
    pub team_abbrev: Option<String>,
    pub position: Option<String>,
    pub goals: Option<u32>,
    pub assists: Option<u32>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BoxscoreTeam {
    pub abbrev: String,
    pub name: Option<TeamName>,
    pub score: Option<u32>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScoringPeriod {
    pub period_descriptor: PeriodDescriptor,
    pub goals: Vec<Goal>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Goal {
    pub time_in_period: String,
    pub team_abbrev: NameField,
    pub first_name: Option<NameField>,
    pub last_name: Option<NameField>,
    pub strength: Option<String>,
    pub assists: Vec<Assist>,
    pub goals_to_date: Option<u32>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Assist {
    pub first_name: Option<NameField>,
    pub last_name: Option<NameField>,
    pub assists_to_date: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The live endpoint spells this field `startTimeUTC`. Under a plain
    /// `rename_all = "camelCase"` it silently deserialized as `None` and every
    /// upcoming game rendered as "TBD".
    #[test]
    fn game_parses_the_uppercase_start_time_field() {
        let json = r#"{
            "id": 2025021000,
            "startTimeUTC": "2026-03-10T23:00:00Z",
            "gameState": "FUT",
            "awayTeam": { "abbrev": "TOR", "score": 0 },
            "homeTeam": { "abbrev": "MTL", "score": 0 },
            "periodDescriptor": { "number": 4, "periodType": "OT" },
            "gameOutcome": { "lastPeriodType": "OT" }
        }"#;
        let game: Game = serde_json::from_str(json).expect("game should parse");
        assert_eq!(game.start_time_utc.as_deref(), Some("2026-03-10T23:00:00Z"));
        assert_eq!(
            game.period_descriptor.map(|p| p.label()).as_deref(),
            Some("OT")
        );
    }

    /// Optional fields the score endpoint omits for scheduled games must not
    /// fail the whole response.
    #[test]
    fn game_tolerates_missing_optional_fields() {
        let json = r#"{
            "id": 1,
            "gameState": "FUT",
            "awayTeam": { "abbrev": "TOR" },
            "homeTeam": { "abbrev": "MTL" }
        }"#;
        let game: Game = serde_json::from_str(json).expect("sparse game should parse");
        assert!(game.start_time_utc.is_none());
        assert!(game.clock.is_none());
        assert!(game.away_team.score.is_none());
    }

    /// Counting stats come back as integers and faceoff/TOI as floats; both
    /// have to land in the same field.
    #[test]
    fn stat_leader_value_accepts_integers_and_floats() {
        let int: StatLeader =
            serde_json::from_str(r#"{ "firstName": {"default": "Connor"}, "value": 138 }"#)
                .expect("integer value should parse");
        assert_eq!(int.value, Some(138.0));

        let float: StatLeader =
            serde_json::from_str(r#"{ "lastName": {"default": "Giroux"}, "value": 0.630788 }"#)
                .expect("float value should parse");
        assert_eq!(float.value, Some(0.630788));
    }
}

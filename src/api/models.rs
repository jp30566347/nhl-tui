use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct ScoreResponse {
    pub games: Vec<Game>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Game {
    pub id: u64,
    pub start_time_utc: Option<String>,
    pub game_state: String,
    pub away_team: TeamScore,
    pub home_team: TeamScore,
    pub game_outcome: Option<GameOutcome>,
    pub period: Option<u32>,
    pub clock: Option<GameClock>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TeamScore {
    pub name: Option<TeamName>,
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
    pub period_type: String,
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
}

#[derive(Debug, Deserialize, Clone)]
pub struct TeamAbbrev {
    pub default: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleResponse {
    pub game_week: Vec<GameDay>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GameDay {
    pub date: String,
    pub day_abbrev: String,
    pub number_of_games: u32,
    pub games: Vec<ScheduleGame>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleGame {
    pub id: u64,
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
    pub common_name: Option<TeamName>,
    pub score: Option<u32>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StatLeader {
    pub id: u32,
    pub first_name: Option<NameField>,
    pub last_name: Option<NameField>,
    pub position: Option<String>,
    pub team_abbrev: Option<String>,
    pub value: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct NameField {
    pub default: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BoxscoreResponse {
    pub id: u64,
    pub game_state: String,
    pub away_team: BoxscoreTeam,
    pub home_team: BoxscoreTeam,
    pub clock: Option<GameClock>,
    pub summary: Option<Summary>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Summary {
    pub scoring: Option<Vec<ScoringPeriod>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BoxscoreTeam {
    pub id: u32,
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
    pub team_abbrev: AbbrevField,
    pub first_name: Option<NameField>,
    pub last_name: Option<NameField>,
    pub goal_modifier: Option<String>,
    pub strength: Option<String>,
    pub assists: Vec<Assist>,
    pub goals_to_date: Option<u32>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AbbrevField {
    pub default: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Assist {
    pub first_name: Option<NameField>,
    pub last_name: Option<NameField>,
    pub assists_to_date: Option<u32>,
}

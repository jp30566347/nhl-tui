use std::collections::HashMap;

use chrono::{Local, NaiveDate};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc::UnboundedSender;

use crate::action::Action;
use crate::api::models::*;
use crate::api::NhlClient;

const LEADER_LIMIT: u32 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Scores,
    Standings,
    Schedule,
    Leaders,
}

impl Tab {
    pub const ALL: [Tab; 4] = [Tab::Scores, Tab::Standings, Tab::Schedule, Tab::Leaders];

    pub fn from_index(i: usize) -> Self {
        Self::ALL.get(i).copied().unwrap_or(Tab::Scores)
    }

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|t| *t == self).unwrap_or(0)
    }

    pub fn title(self) -> &'static str {
        match self {
            Tab::Scores => "[1] Scores",
            Tab::Standings => "[2] Standings",
            Tab::Schedule => "[3] Schedule",
            Tab::Leaders => "[4] Leaders",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StandingsFilter {
    Conference,
    Division,
    League,
}

impl StandingsFilter {
    pub const ALL: [StandingsFilter; 3] = [
        StandingsFilter::Conference,
        StandingsFilter::Division,
        StandingsFilter::League,
    ];

    pub fn next(self) -> Self {
        cycle(&Self::ALL, self, 1)
    }

    pub fn prev(self) -> Self {
        cycle(&Self::ALL, self, -1)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            StandingsFilter::Conference => "Conference",
            StandingsFilter::Division => "Division",
            StandingsFilter::League => "League",
        }
    }
}

/// The categories the stats-leaders endpoint actually serves. Anything else
/// comes back as HTTP 400, so this list is the source of truth for both the
/// tab cycle and the request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaderCategory {
    Points,
    Goals,
    Assists,
    PlusMinus,
    PenaltyMins,
    PowerPlayGoals,
    ShorthandedGoals,
    Faceoffs,
    TimeOnIce,
}

impl LeaderCategory {
    pub const ALL: [LeaderCategory; 9] = [
        LeaderCategory::Points,
        LeaderCategory::Goals,
        LeaderCategory::Assists,
        LeaderCategory::PlusMinus,
        LeaderCategory::PenaltyMins,
        LeaderCategory::PowerPlayGoals,
        LeaderCategory::ShorthandedGoals,
        LeaderCategory::Faceoffs,
        LeaderCategory::TimeOnIce,
    ];

    pub fn next(self) -> Self {
        cycle(&Self::ALL, self, 1)
    }

    pub fn prev(self) -> Self {
        cycle(&Self::ALL, self, -1)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            LeaderCategory::Points => "Points",
            LeaderCategory::Goals => "Goals",
            LeaderCategory::Assists => "Assists",
            LeaderCategory::PlusMinus => "+/-",
            LeaderCategory::PenaltyMins => "PIM",
            LeaderCategory::PowerPlayGoals => "PP Goals",
            LeaderCategory::ShorthandedGoals => "SH Goals",
            LeaderCategory::Faceoffs => "Faceoff %",
            LeaderCategory::TimeOnIce => "TOI/GP",
        }
    }

    pub fn api_key(self) -> &'static str {
        match self {
            LeaderCategory::Points => "points",
            LeaderCategory::Goals => "goals",
            LeaderCategory::Assists => "assists",
            LeaderCategory::PlusMinus => "plusMinus",
            LeaderCategory::PenaltyMins => "penaltyMins",
            LeaderCategory::PowerPlayGoals => "goalsPp",
            LeaderCategory::ShorthandedGoals => "goalsSh",
            LeaderCategory::Faceoffs => "faceoffLeaders",
            LeaderCategory::TimeOnIce => "toi",
        }
    }

    /// Counting stats arrive as whole numbers, faceoffs as a ratio, and time
    /// on ice as seconds.
    pub fn format_value(self, value: f64) -> String {
        match self {
            LeaderCategory::Faceoffs => format!("{:.1}%", value * 100.0),
            LeaderCategory::TimeOnIce => {
                let secs = value.max(0.0).round() as u64;
                format!("{}:{:02}", secs / 60, secs % 60)
            }
            _ => format!("{}", value.round() as i64),
        }
    }
}

fn cycle<T: Copy + PartialEq>(all: &[T], current: T, step: isize) -> T {
    let len = all.len();
    let idx = all.iter().position(|v| *v == current).unwrap_or(0);
    all[(idx as isize + step).rem_euclid(len as isize) as usize]
}

/// One round of network results, handed back to the app from a background task.
#[derive(Debug)]
pub struct Fetched {
    /// Identifies the request that produced this. Results from a superseded
    /// request (the user changed the date mid-flight) are dropped.
    pub request_id: u64,
    pub scores: Result<ScoreResponse, String>,
    pub standings: Result<StandingsResponse, String>,
    pub schedule: Result<ScheduleResponse, String>,
    pub leaders: Result<HashMap<String, Vec<StatLeader>>, String>,
    pub boxscore: Option<Result<BoxscoreResponse, String>>,
}

pub struct App {
    pub should_quit: bool,
    pub active_tab: Tab,
    pub current_date: NaiveDate,
    pub favorite_team: Option<String>,

    pub scores: Option<ScoreResponse>,
    pub standings: Option<StandingsResponse>,
    pub schedule: Option<ScheduleResponse>,
    pub leaders: HashMap<String, Vec<StatLeader>>,
    pub boxscore: Option<BoxscoreResponse>,

    pub standings_filter: StandingsFilter,
    pub leader_category: LeaderCategory,
    pub scores_scroll: usize,
    pub standings_scroll: usize,
    pub schedule_scroll: usize,
    pub leaders_scroll: usize,

    pub show_boxscore: bool,
    pub selected_game_id: Option<u64>,

    pub loading: bool,
    pub last_updated: Option<chrono::DateTime<Local>>,
    /// Most recent error, shown in the status bar. Printing to stderr would
    /// corrupt the alternate screen we are drawing on.
    pub error: Option<String>,
    /// Last seen combined score per game, used to detect that a favourite
    /// team's game changed while we were away.
    last_scores: HashMap<u64, u32>,

    request_id: u64,
    client: NhlClient,
}

impl App {
    pub fn new(favorite_team: Option<String>, start_tab: usize) -> Self {
        Self {
            should_quit: false,
            active_tab: Tab::from_index(start_tab),
            current_date: Local::now().date_naive(),
            favorite_team: favorite_team.map(|t| t.to_uppercase()),
            scores: None,
            standings: None,
            schedule: None,
            leaders: HashMap::new(),
            boxscore: None,
            standings_filter: StandingsFilter::Conference,
            leader_category: LeaderCategory::Points,
            scores_scroll: 0,
            standings_scroll: 0,
            schedule_scroll: 0,
            leaders_scroll: 0,
            show_boxscore: false,
            selected_game_id: None,
            loading: false,
            last_updated: None,
            error: None,
            last_scores: HashMap::new(),
            request_id: 0,
            client: NhlClient::new(),
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        if self.show_boxscore {
            if matches!(key.code, KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q')) {
                self.show_boxscore = false;
                self.boxscore = None;
            }
            return None;
        }

        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true
            }
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('r') => return Some(Action::Refresh),
            KeyCode::Char(c @ '1'..='4') => {
                self.active_tab = Tab::from_index(c as usize - '1' as usize)
            }
            KeyCode::Left | KeyCode::Char('h') => match self.active_tab {
                Tab::Scores | Tab::Schedule => {
                    self.current_date -= chrono::Duration::days(1);
                    self.reset_scroll();
                    return Some(Action::Refresh);
                }
                Tab::Standings => self.standings_filter = self.standings_filter.prev(),
                Tab::Leaders => {
                    self.leader_category = self.leader_category.prev();
                    self.leaders_scroll = 0;
                }
            },
            KeyCode::Right | KeyCode::Char('l') => match self.active_tab {
                Tab::Scores | Tab::Schedule => {
                    self.current_date += chrono::Duration::days(1);
                    self.reset_scroll();
                    return Some(Action::Refresh);
                }
                Tab::Standings => self.standings_filter = self.standings_filter.next(),
                Tab::Leaders => {
                    self.leader_category = self.leader_category.next();
                    self.leaders_scroll = 0;
                }
            },
            KeyCode::Up | KeyCode::Char('k') => {
                let scroll = self.scroll_mut();
                *scroll = scroll.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max = self.row_count();
                let scroll = self.scroll_mut();
                if *scroll + 1 < max {
                    *scroll += 1;
                }
            }
            KeyCode::Home | KeyCode::Char('g') => *self.scroll_mut() = 0,
            KeyCode::End | KeyCode::Char('G') => {
                let last = self.row_count().saturating_sub(1);
                *self.scroll_mut() = last;
            }
            KeyCode::Enter if self.active_tab == Tab::Scores => {
                if let Some(game) = self.selected_game() {
                    self.selected_game_id = Some(game.id);
                    self.show_boxscore = true;
                    // Fetch the boxscore now rather than waiting out the
                    // refresh interval.
                    return Some(Action::Refresh);
                }
            }
            _ => {}
        }
        None
    }

    /// Number of selectable rows on the active tab.
    fn row_count(&self) -> usize {
        match self.active_tab {
            Tab::Scores => self.scores.as_ref().map_or(0, |s| s.games.len()),
            Tab::Standings => self.filtered_standings().len(),
            Tab::Schedule => self
                .schedule
                .as_ref()
                .map_or(0, |s| s.game_week.iter().map(|d| d.games.len()).sum()),
            Tab::Leaders => self.current_leaders().len(),
        }
    }

    fn scroll_mut(&mut self) -> &mut usize {
        match self.active_tab {
            Tab::Scores => &mut self.scores_scroll,
            Tab::Standings => &mut self.standings_scroll,
            Tab::Schedule => &mut self.schedule_scroll,
            Tab::Leaders => &mut self.leaders_scroll,
        }
    }

    fn reset_scroll(&mut self) {
        self.scores_scroll = 0;
        self.schedule_scroll = 0;
    }

    /// Keeps selections pointing at a row that still exists after a refresh
    /// returns a different number of games.
    fn clamp_scroll(&mut self) {
        // Lengths are read up front: each getter borrows `self` immutably.
        let lengths = [
            self.scores.as_ref().map_or(0, |s| s.games.len()),
            self.filtered_standings().len(),
            self.schedule
                .as_ref()
                .map_or(0, |s| s.game_week.iter().map(|d| d.games.len()).sum()),
            self.current_leaders().len(),
        ];
        let scrolls = [
            &mut self.scores_scroll,
            &mut self.standings_scroll,
            &mut self.schedule_scroll,
            &mut self.leaders_scroll,
        ];
        for (scroll, len) in scrolls.into_iter().zip(lengths) {
            *scroll = (*scroll).min(len.saturating_sub(1));
        }
    }

    pub fn selected_game(&self) -> Option<&Game> {
        self.scores
            .as_ref()
            .and_then(|s| s.games.get(self.scores_scroll))
    }

    pub fn date_str(&self) -> String {
        self.current_date.format("%Y-%m-%d").to_string()
    }

    pub fn is_today(&self) -> bool {
        self.current_date == Local::now().date_naive()
    }

    pub fn filtered_standings(&self) -> Vec<&Standing> {
        let Some(standings) = self.standings.as_ref().map(|s| &s.standings) else {
            return Vec::new();
        };
        let mut rows: Vec<&Standing> = standings.iter().collect();
        match self.standings_filter {
            StandingsFilter::League => {
                rows.sort_by_key(|s| s.league_sequence.unwrap_or(u32::MAX));
            }
            StandingsFilter::Conference => {
                // Eastern before Western, each in conference order.
                rows.sort_by_key(|s| {
                    (
                        s.conference_name.clone(),
                        s.conference_sequence.unwrap_or(u32::MAX),
                    )
                });
            }
            StandingsFilter::Division => {
                rows.sort_by_key(|s| {
                    (
                        s.division_name.clone(),
                        s.division_sequence.unwrap_or(u32::MAX),
                    )
                });
            }
        }
        rows
    }

    /// The heading a row sits under, or `None` when the active filter is flat.
    pub fn standings_group<'a>(&self, standing: &'a Standing) -> Option<&'a str> {
        match self.standings_filter {
            StandingsFilter::Conference => Some(&standing.conference_name),
            StandingsFilter::Division => Some(&standing.division_name),
            StandingsFilter::League => None,
        }
    }

    pub fn current_leaders(&self) -> &[StatLeader] {
        self.leaders
            .get(self.leader_category.api_key())
            .map_or(&[], |v| v.as_slice())
    }

    pub fn is_favorite_team(&self, abbrev: &str) -> bool {
        self.favorite_team.as_deref() == Some(abbrev)
    }

    /// True when a favourite team's game has scored since the last refresh.
    ///
    /// A game we have not seen before never alerts, so opening the app during
    /// a 3-1 game is silent; only a change from a known score rings.
    fn check_score_alerts(&mut self) -> bool {
        let (Some(scores), Some(fav)) = (&self.scores, &self.favorite_team) else {
            return false;
        };
        let mut alert = false;
        let mut seen = HashMap::new();
        for game in &scores.games {
            if game.home_team.abbrev != *fav && game.away_team.abbrev != *fav {
                continue;
            }
            let total = game.home_team.score.unwrap_or(0) + game.away_team.score.unwrap_or(0);
            if self
                .last_scores
                .get(&game.id)
                .is_some_and(|prev| total > *prev)
            {
                alert = true;
            }
            seen.insert(game.id, total);
        }
        self.last_scores = seen;
        alert
    }

    /// Starts a fetch in the background and returns immediately, so the UI
    /// keeps responding to keys while the network call is in flight.
    pub fn spawn_fetch(&mut self, tx: UnboundedSender<Action>) {
        self.request_id += 1;
        self.loading = true;

        let request_id = self.request_id;
        let client = self.client.clone();
        let date = self.date_str();
        let boxscore_id = self
            .show_boxscore
            .then_some(self.selected_game_id)
            .flatten();

        tokio::spawn(async move {
            let (scores, standings, schedule, leaders) = tokio::join!(
                client.get_scores(&date),
                client.get_standings(),
                client.get_schedule(&date),
                client.get_leaders(LEADER_LIMIT),
            );
            let boxscore = match boxscore_id {
                Some(id) => Some(client.get_boxscore(id).await.map_err(|e| e.to_string())),
                None => None,
            };
            let _ = tx.send(Action::Fetched(Box::new(Fetched {
                request_id,
                scores: scores.map_err(|e| e.to_string()),
                standings: standings.map_err(|e| e.to_string()),
                schedule: schedule.map_err(|e| e.to_string()),
                leaders: leaders.map_err(|e| e.to_string()),
                boxscore,
            })));
        });
    }

    /// Folds a completed fetch into the app. Returns true if the favourite
    /// team scored.
    pub fn apply_fetch(&mut self, fetched: Fetched) -> bool {
        if fetched.request_id != self.request_id {
            return false; // Superseded by a later request.
        }
        self.loading = false;
        self.last_updated = Some(Local::now());

        let mut errors = Vec::new();
        // A closure would be monomorphic over one payload type; each slot
        // holds a different one.
        macro_rules! take {
            ($slot:expr, $result:expr) => {
                match $result {
                    Ok(value) => $slot = Some(value),
                    Err(e) => errors.push(e),
                }
            };
        }
        take!(self.scores, fetched.scores);
        take!(self.standings, fetched.standings);
        take!(self.schedule, fetched.schedule);

        match fetched.leaders {
            Ok(leaders) => self.leaders = leaders,
            Err(e) => errors.push(e),
        }
        match fetched.boxscore {
            Some(Ok(boxscore)) => self.boxscore = Some(boxscore),
            Some(Err(e)) => errors.push(e),
            None => {}
        }

        self.error = errors.first().cloned();
        self.clamp_scroll();
        self.check_score_alerts()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        App::new(Some("tor".into()), 0)
    }

    fn game(id: u64, away: (&str, u32), home: (&str, u32)) -> Game {
        let team = |(abbrev, score): (&str, u32)| TeamScore {
            abbrev: abbrev.to_string(),
            score: Some(score),
        };
        Game {
            id,
            start_time_utc: None,
            game_state: "LIVE".into(),
            away_team: team(away),
            home_team: team(home),
            game_outcome: None,
            period: None,
            period_descriptor: None,
            clock: None,
        }
    }

    #[test]
    fn favorite_team_is_normalized_to_uppercase() {
        assert!(app().is_favorite_team("TOR"));
        assert!(!app().is_favorite_team("tor"));
    }

    #[test]
    fn categories_cycle_in_both_directions() {
        let first = LeaderCategory::ALL[0];
        let last = LeaderCategory::ALL[LeaderCategory::ALL.len() - 1];
        assert_eq!(first.prev(), last);
        assert_eq!(last.next(), first);

        let mut c = first;
        for _ in 0..LeaderCategory::ALL.len() {
            c = c.next();
        }
        assert_eq!(c, first, "a full cycle returns to the start");
    }

    #[test]
    fn standings_filters_cycle() {
        assert_eq!(StandingsFilter::Conference.prev(), StandingsFilter::League);
        assert_eq!(StandingsFilter::League.next(), StandingsFilter::Conference);
    }

    #[test]
    fn leader_values_are_formatted_per_category() {
        assert_eq!(LeaderCategory::Points.format_value(138.0), "138");
        assert_eq!(LeaderCategory::Faceoffs.format_value(0.630788), "63.1%");
        assert_eq!(LeaderCategory::TimeOnIce.format_value(1664.2568), "27:44");
    }

    #[test]
    fn period_labels_cover_overtime_and_shootout() {
        let pd = |number, period_type: Option<&str>| PeriodDescriptor {
            number,
            period_type: period_type.map(str::to_string),
        };
        assert_eq!(pd(1, Some("REG")).label(), "1st");
        assert_eq!(pd(4, Some("OT")).label(), "OT");
        assert_eq!(pd(5, Some("OT")).label(), "2OT");
        assert_eq!(pd(5, Some("SO")).label(), "SO");
    }

    #[test]
    fn first_sighting_of_a_game_does_not_alert() {
        let mut app = app();
        app.scores = Some(ScoreResponse {
            games: vec![game(1, ("TOR", 3), ("MTL", 1))],
        });
        assert!(!app.check_score_alerts(), "opening mid-game must be silent");
    }

    #[test]
    fn a_goal_by_a_tracked_game_alerts_once() {
        let mut app = app();
        app.scores = Some(ScoreResponse {
            games: vec![game(1, ("TOR", 3), ("MTL", 1))],
        });
        app.check_score_alerts();

        app.scores = Some(ScoreResponse {
            games: vec![game(1, ("TOR", 4), ("MTL", 1))],
        });
        assert!(app.check_score_alerts());
        assert!(!app.check_score_alerts(), "same score must not re-alert");
    }

    #[test]
    fn games_without_the_favorite_team_never_alert() {
        let mut app = app();
        app.scores = Some(ScoreResponse {
            games: vec![game(1, ("BOS", 1), ("MTL", 1))],
        });
        app.check_score_alerts();
        app.scores = Some(ScoreResponse {
            games: vec![game(1, ("BOS", 2), ("MTL", 1))],
        });
        assert!(!app.check_score_alerts());
    }

    #[test]
    fn selection_is_clamped_when_a_refresh_returns_fewer_games() {
        let mut app = app();
        app.scores = Some(ScoreResponse {
            games: (0..5).map(|i| game(i, ("TOR", 0), ("MTL", 0))).collect(),
        });
        app.scores_scroll = 4;

        app.scores = Some(ScoreResponse {
            games: vec![game(0, ("TOR", 0), ("MTL", 0))],
        });
        app.clamp_scroll();
        assert_eq!(app.scores_scroll, 0);
        assert!(app.selected_game().is_some());
    }

    #[test]
    fn stale_results_are_discarded() {
        let mut app = app();
        app.request_id = 7;
        let stale = Fetched {
            request_id: 6,
            scores: Ok(ScoreResponse {
                games: vec![game(1, ("TOR", 9), ("MTL", 0))],
            }),
            standings: Err("x".into()),
            schedule: Err("x".into()),
            leaders: Err("x".into()),
            boxscore: None,
        };
        app.apply_fetch(stale);
        assert!(app.scores.is_none(), "a superseded response must not land");
    }

    #[test]
    fn changing_date_resets_selection_and_requests_a_refresh() {
        let mut app = app();
        app.scores_scroll = 3;
        let start = app.current_date;
        let action = app.handle_key(KeyEvent::from(KeyCode::Char('l')));
        assert!(matches!(action, Some(Action::Refresh)));
        assert_eq!(app.current_date, start + chrono::Duration::days(1));
        assert_eq!(app.scores_scroll, 0);
    }
}

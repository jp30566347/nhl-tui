use chrono::{Local, NaiveDate};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::HashMap;

use crate::action::Action;
use crate::api::models::*;
use crate::api::NhlClient;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tab {
    Scores = 0,
    Standings = 1,
    Schedule = 2,
    Leaders = 3,
}

impl Tab {
    pub fn from_index(i: usize) -> Self {
        match i { 0 => Tab::Scores, 1 => Tab::Standings, 2 => Tab::Schedule, 3 => Tab::Leaders, _ => Tab::Scores }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StandingsFilter {
    Conference,
    Division,
    League,
}

impl StandingsFilter {
    pub fn next(self) -> Self {
        match self { StandingsFilter::Conference => StandingsFilter::Division, StandingsFilter::Division => StandingsFilter::League, StandingsFilter::League => StandingsFilter::Conference }
    }
    pub fn prev(self) -> Self {
        match self { StandingsFilter::Conference => StandingsFilter::League, StandingsFilter::Division => StandingsFilter::Conference, StandingsFilter::League => StandingsFilter::Division }
    }
    pub fn as_str(&self) -> &'static str {
        match self { StandingsFilter::Conference => "Conference", StandingsFilter::Division => "Division", StandingsFilter::League => "League" }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LeaderCategory {
    Points,
    Goals,
    Assists,
    PlusMinus,
    Pim,
    Shots,
}

impl LeaderCategory {
    pub fn next(self) -> Self {
        match self {
            LeaderCategory::Points => LeaderCategory::Goals,
            LeaderCategory::Goals => LeaderCategory::Assists,
            LeaderCategory::Assists => LeaderCategory::PlusMinus,
            LeaderCategory::PlusMinus => LeaderCategory::Pim,
            LeaderCategory::Pim => LeaderCategory::Shots,
            LeaderCategory::Shots => LeaderCategory::Points,
        }
    }
    pub fn prev(self) -> Self {
        match self {
            LeaderCategory::Points => LeaderCategory::Shots,
            LeaderCategory::Goals => LeaderCategory::Points,
            LeaderCategory::Assists => LeaderCategory::Goals,
            LeaderCategory::PlusMinus => LeaderCategory::Assists,
            LeaderCategory::Pim => LeaderCategory::PlusMinus,
            LeaderCategory::Shots => LeaderCategory::Pim,
        }
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            LeaderCategory::Points => "Points", LeaderCategory::Goals => "Goals",
            LeaderCategory::Assists => "Assists", LeaderCategory::PlusMinus => "+/-",
            LeaderCategory::Pim => "PIM", LeaderCategory::Shots => "Shots",
        }
    }
    pub fn api_key(&self) -> &'static str {
        match self {
            LeaderCategory::Points => "points", LeaderCategory::Goals => "goals",
            LeaderCategory::Assists => "assists", LeaderCategory::PlusMinus => "plusMinus",
            LeaderCategory::Pim => "pim", LeaderCategory::Shots => "shots",
        }
    }
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
    pub last_scores: std::collections::HashMap<u64, u32>,

    pub client: NhlClient,
}

impl App {
    pub fn new(favorite_team: Option<String>, start_tab: usize) -> Self {
        Self {
            should_quit: false,
            active_tab: Tab::from_index(start_tab),
            current_date: Local::now().naive_local().date(),
            favorite_team: favorite_team.map(|t| t.to_uppercase()),
            scores: None, standings: None, schedule: None,
            leaders: HashMap::new(), boxscore: None,
            standings_filter: StandingsFilter::Conference,
            leader_category: LeaderCategory::Points,
            scores_scroll: 0, standings_scroll: 0, schedule_scroll: 0, leaders_scroll: 0,
            show_boxscore: false, selected_game_id: None,
            loading: false, last_updated: None,
            last_scores: std::collections::HashMap::new(),
            client: NhlClient::new(),
        }
    }

    pub async fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        if self.show_boxscore {
            if matches!(key.code, KeyCode::Esc | KeyCode::Enter) {
                self.show_boxscore = false;
                self.boxscore = None;
            }
            return None;
        }

        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Esc if key.modifiers.contains(KeyModifiers::CONTROL) => self.should_quit = true,
            KeyCode::Char('r') => return Some(Action::Tick),
            KeyCode::Char('1') => self.active_tab = Tab::Scores,
            KeyCode::Char('2') => self.active_tab = Tab::Standings,
            KeyCode::Char('3') => self.active_tab = Tab::Schedule,
            KeyCode::Char('4') => self.active_tab = Tab::Leaders,
            KeyCode::Left | KeyCode::Char('h') => match self.active_tab {
                Tab::Scores | Tab::Schedule => self.prev_date(),
                Tab::Standings => self.standings_filter = self.standings_filter.prev(),
                Tab::Leaders => self.leader_category = self.leader_category.prev(),
            },
            KeyCode::Right | KeyCode::Char('l') => match self.active_tab {
                Tab::Scores | Tab::Schedule => self.next_date(),
                Tab::Standings => self.standings_filter = self.standings_filter.next(),
                Tab::Leaders => self.leader_category = self.leader_category.next(),
            },
            KeyCode::Up | KeyCode::Char('k') => {
                let scroll = match self.active_tab {
                    Tab::Scores => &mut self.scores_scroll,
                    Tab::Standings => &mut self.standings_scroll,
                    Tab::Schedule => &mut self.schedule_scroll,
                    Tab::Leaders => &mut self.leaders_scroll,
                };
                if *scroll > 0 { *scroll -= 1; }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max = match self.active_tab {
                    Tab::Scores => self.scores.as_ref().map(|s| s.games.len()).unwrap_or(0),
                    Tab::Standings => self.filtered_standings().len(),
                    Tab::Schedule => self.schedule.as_ref().map(|s| s.game_week.iter().flat_map(|d| d.games.iter()).count()).unwrap_or(0),
                    Tab::Leaders => self.current_leaders().len(),
                };
                let scroll = match self.active_tab {
                    Tab::Scores => &mut self.scores_scroll,
                    Tab::Standings => &mut self.standings_scroll,
                    Tab::Schedule => &mut self.schedule_scroll,
                    Tab::Leaders => &mut self.leaders_scroll,
                };
                if *scroll + 1 < max { *scroll += 1; }
            }
            KeyCode::Enter => {
                if self.active_tab == Tab::Scores {
                    if let Some(game) = self.scores.as_ref().and_then(|s| s.games.get(self.scores_scroll)) {
                        self.selected_game_id = Some(game.id);
                        self.show_boxscore = true;
                    }
                }
            }
            _ => {}
        }
        None
    }

    fn prev_date(&mut self) { self.current_date = self.current_date - chrono::Duration::days(1); }
    fn next_date(&mut self) { self.current_date = self.current_date + chrono::Duration::days(1); }
    pub fn date_str(&self) -> String { self.current_date.format("%Y-%m-%d").to_string() }
    pub fn is_today(&self) -> bool { self.current_date == Local::now().naive_local().date() }

    pub fn filtered_standings(&self) -> Vec<&Standing> {
        let standings = match &self.standings { Some(s) => &s.standings, None => return vec![] };
        match self.standings_filter {
            StandingsFilter::League => standings.iter().collect(),
            StandingsFilter::Conference => {
                let mut east: Vec<&Standing> = standings.iter().filter(|s| s.conference_name == "Eastern").collect();
                let mut west: Vec<&Standing> = standings.iter().filter(|s| s.conference_name == "Western").collect();
                east.sort_by_key(|s| s.conference_sequence.unwrap_or(999));
                west.sort_by_key(|s| s.conference_sequence.unwrap_or(999));
                east.into_iter().chain(west).collect()
            }
            StandingsFilter::Division => {
                let mut divs: Vec<&Standing> = standings.iter().collect();
                divs.sort_by(|a, b| a.division_name.cmp(&b.division_name).then_with(|| a.division_sequence.unwrap_or(999).cmp(&b.division_sequence.unwrap_or(999))));
                divs
            }
        }
    }

    pub fn current_leaders(&self) -> &[StatLeader] {
        self.leaders.get(self.leader_category.api_key()).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn is_favorite_team(&self, abbrev: &str) -> bool {
        self.favorite_team.as_ref().map(|ft| ft == abbrev).unwrap_or(false)
    }

    pub fn check_score_alerts(&mut self) -> bool {
        let (scores, fav) = match (&self.scores, &self.favorite_team) {
            (Some(s), Some(f)) => (s, f.clone()),
            _ => return false,
        };
        let mut alert = false;
        for game in &scores.games {
            if game.home_team.abbrev == fav || game.away_team.abbrev == fav {
                let total = game.home_team.score.unwrap_or(0) + game.away_team.score.unwrap_or(0);
                let prev = self.last_scores.get(&game.id).copied().unwrap_or(0);
                if total > prev { alert = true; }
                self.last_scores.insert(game.id, total);
            }
        }
        alert
    }

    pub async fn fetch_all(&mut self) {
        self.loading = true;
        let date = self.date_str();

        let scores_fut = self.client.get_scores(&date);
        let standings_fut = self.client.get_standings();
        let schedule_fut = self.client.get_schedule(&date);
        let leaders_points = self.client.get_leaders("points", 10);
        let leaders_goals = self.client.get_leaders("goals", 10);
        let leaders_assists = self.client.get_leaders("assists", 10);

        let (scores_res, standings_res, schedule_res, pts_res, goals_res, assists_res) =
            tokio::join!(scores_fut, standings_fut, schedule_fut, leaders_points, leaders_goals, leaders_assists);

        match scores_res {
            Ok(s) => self.scores = Some(s),
            Err(e) => eprintln!("Scores: {}", e),
        }
        match standings_res {
            Ok(s) => self.standings = Some(s),
            Err(e) => eprintln!("Standings: {}", e),
        }
        match schedule_res {
            Ok(s) => self.schedule = Some(s),
            Err(e) => eprintln!("Schedule: {}", e),
        }
        if let Ok(l) = pts_res { self.leaders.insert("points".into(), l); }
        if let Ok(l) = goals_res { self.leaders.insert("goals".into(), l); }
        if let Ok(l) = assists_res { self.leaders.insert("assists".into(), l); }

        if let Some(game_id) = self.selected_game_id {
            if self.show_boxscore {
                if let Ok(boxscore) = self.client.get_boxscore(game_id).await {
                    self.boxscore = Some(boxscore);
                }
            }
        }

        self.loading = false;
        self.last_updated = Some(Local::now());
    }
}

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Cell, Clear, HighlightSpacing, Paragraph, Row, Table, TableState, Tabs,
    },
    Frame,
};

use crate::api::models::PeriodDescriptor;
use crate::app::{App, StandingsFilter, Tab};

const SELECTED_BG: Color = Color::DarkGray;
const SELECTED_STYLE: Style = Style::new()
    .bg(SELECTED_BG)
    .fg(Color::White)
    .add_modifier(Modifier::BOLD);
const HEADING: Style = Style::new().fg(Color::Cyan);
const MUTED: Style = Style::new().fg(Color::DarkGray);
const FAVORITE: Style = Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD);

pub fn draw(f: &mut Frame, app: &App) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(area);

    draw_tabs(f, app, chunks[0]);
    match app.active_tab {
        Tab::Scores => draw_scores(f, app, chunks[1]),
        Tab::Standings => draw_standings(f, app, chunks[1]),
        Tab::Schedule => draw_schedule(f, app, chunks[1]),
        Tab::Leaders => draw_leaders(f, app, chunks[1]),
    }
    draw_status(f, app, chunks[2]);

    if app.show_boxscore {
        draw_boxscore_overlay(f, app, area);
    }
}

/// Offset that keeps `selected` on screen without storing scroll position
/// between frames: the selection rides the bottom edge once the list is
/// longer than the viewport.
fn scroll_offset(selected: usize, height: usize, total: usize) -> usize {
    if height == 0 || total <= height {
        return 0;
    }
    selected.saturating_sub(height - 1).min(total - height)
}

fn panel(title: String, hint: &'static str) -> Block<'static> {
    Block::default()
        .title(title)
        .title_bottom(hint)
        .borders(Borders::ALL)
}

fn placeholder(f: &mut Frame, area: Rect, text: &str, style: Style) {
    f.render_widget(
        Paragraph::new(text)
            .alignment(Alignment::Center)
            .style(style),
        area,
    );
}

fn date_title(app: &App, name: &str) -> String {
    if app.is_today() {
        format!(" {} {} (Today) ", name, app.date_str())
    } else {
        format!(" {} {} ", name, app.date_str())
    }
}

fn header_row(labels: &[&'static str]) -> Row<'static> {
    Row::new(
        labels
            .iter()
            .map(|l| Cell::from(*l).style(Style::new().bold())),
    )
}

fn draw_tabs(f: &mut Frame, app: &App, area: Rect) {
    let tabs = Tabs::new(Tab::ALL.map(Tab::title).to_vec())
        .block(
            Block::default()
                .title(" NHL Dashboard ")
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(HEADING),
        )
        .select(app.active_tab.index())
        .style(Style::new().fg(Color::White))
        .highlight_style(FAVORITE);
    f.render_widget(tabs, area);
}

fn draw_scores(f: &mut Frame, app: &App, area: Rect) {
    let block = panel(
        date_title(app, "Scores"),
        " \u{25C4} h/Left  |  l/Right \u{25BA}  |  j/k \u{2195}  |  Enter: details ",
    );
    let inner = block.inner(area);
    f.render_widget(block, area);

    let Some(scores) = &app.scores else {
        return placeholder(f, inner, "Loading\u{2026}", Style::new());
    };
    if scores.games.is_empty() {
        return placeholder(f, inner, "No games.", MUTED);
    }

    let lines: Vec<Line> = scores
        .games
        .iter()
        .enumerate()
        .map(|(i, game)| {
            let selected = i == app.scores_scroll;
            let (state, state_style) = game_state_label(game);

            let team_style = |abbrev: &str| {
                if app.is_favorite_team(abbrev) {
                    FAVORITE
                } else {
                    Style::new()
                }
            };
            let score = Style::new().bold();

            let mut spans = vec![
                Span::raw(if selected { "\u{25B8} " } else { "  " }),
                Span::styled(
                    format!("{:>3} ", game.away_team.abbrev),
                    team_style(&game.away_team.abbrev),
                ),
                Span::styled(format!("{:>2}", game.away_team.score.unwrap_or(0)), score),
                Span::raw(" - "),
                Span::styled(format!("{:<2}", game.home_team.score.unwrap_or(0)), score),
                Span::styled(
                    format!("{:<3}", game.home_team.abbrev),
                    team_style(&game.home_team.abbrev),
                ),
                Span::raw("  "),
                Span::styled(state, state_style),
            ];
            if selected {
                spans.push(Span::styled("  \u{21B5} Enter", FAVORITE));
            }
            // The line style is the base that span styles patch, so the
            // selection highlight is set once here instead of on every span.
            let base = if selected {
                Style::new().bg(SELECTED_BG)
            } else {
                Style::new()
            };
            Line::from(spans).style(base)
        })
        .collect();

    let offset = scroll_offset(app.scores_scroll, inner.height as usize, lines.len());
    f.render_widget(Paragraph::new(lines).scroll((offset as u16, 0)), inner);
}

fn game_state_label(game: &crate::api::models::Game) -> (String, Style) {
    match game.game_state.as_str() {
        // CRIT is a late, close game; the API treats it as live.
        "LIVE" | "CRIT" => {
            let period = game
                .period_descriptor
                .as_ref()
                .map(PeriodDescriptor::label)
                .unwrap_or_else(|| game.period.unwrap_or(0).to_string());
            let clock = game
                .clock
                .as_ref()
                .and_then(|c| c.time_remaining.as_deref())
                .unwrap_or("");
            (
                format!("LIVE {period} {clock}").trim_end().to_string(),
                Style::new().fg(Color::Green),
            )
        }
        "FINAL" | "OFF" => {
            let label = match game
                .game_outcome
                .as_ref()
                .and_then(|o| o.last_period_type.as_deref())
            {
                Some("OT") => "FINAL/OT",
                Some("SO") => "FINAL/SO",
                _ => "FINAL",
            };
            (label.to_string(), MUTED)
        }
        "FUT" | "PRE" => (
            start_time(game.start_time_utc.as_deref()),
            Style::new().fg(Color::Blue),
        ),
        other => (other.to_string(), Style::new().fg(Color::White)),
    }
}

/// Formats a UTC timestamp in the viewer's local timezone.
fn start_time(utc: Option<&str>) -> String {
    utc.and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
        .map(|d| {
            d.with_timezone(&chrono::Local)
                .format("%-I:%M %p")
                .to_string()
        })
        .unwrap_or_else(|| "TBD".to_string())
}

fn draw_standings(f: &mut Frame, app: &App, area: Rect) {
    let block = panel(
        format!(" Standings [{}] ", app.standings_filter.as_str()),
        " \u{25C4} h  |  l \u{25BA}  |  j/k \u{2195} ",
    );
    let inner = block.inner(area);
    f.render_widget(block, area);

    let filtered = app.filtered_standings();
    if filtered.is_empty() {
        return placeholder(f, inner, "Loading\u{2026}", Style::new());
    }

    let mut rows = Vec::with_capacity(filtered.len() + 4);
    let mut selected_row = 0;
    let mut current_group: Option<String> = None;

    for (i, s) in filtered.iter().enumerate() {
        if let Some(group) = app.standings_group(s) {
            if current_group.as_deref() != Some(group) {
                current_group = Some(group.to_string());
                // The label goes in the Team column; the first column is only
                // three cells wide and would clip it.
                rows.push(Row::new(vec![
                    Cell::from(""),
                    Cell::from(format!("\u{2500} {group} \u{2500}")).style(HEADING),
                ]));
            }
        }

        if i == app.standings_scroll {
            selected_row = rows.len();
        }

        let seq = match app.standings_filter {
            StandingsFilter::League => s.league_sequence,
            StandingsFilter::Conference => s.conference_sequence,
            StandingsFilter::Division => s.division_sequence,
        }
        .unwrap_or(i as u32 + 1);

        let style = if app.is_favorite_team(&s.team_abbrev.default) {
            FAVORITE
        } else {
            Style::new()
        };

        rows.push(
            Row::new(vec![
                seq.to_string(),
                format!("{} ({})", s.team_name.default, s.team_abbrev.default),
                s.games_played.to_string(),
                s.wins.to_string(),
                s.losses.to_string(),
                s.ot_losses.to_string(),
                s.points.to_string(),
                s.goal_for.to_string(),
                s.goal_against.to_string(),
                format!("{:+}", s.goal_differential),
                format!(
                    "{}{}",
                    s.streak_code.as_deref().unwrap_or(""),
                    s.streak_count.unwrap_or(0)
                ),
            ])
            .style(style),
        );
    }

    let table = Table::new(
        rows,
        [
            Constraint::Length(3),
            Constraint::Min(20),
            Constraint::Length(4),
            Constraint::Length(4),
            Constraint::Length(4),
            Constraint::Length(4),
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Length(6),
        ],
    )
    .header(header_row(&[
        "#", "Team", "GP", "W", "L", "OT", "PTS", "GF", "GA", "+/-", "STRK",
    ]))
    .row_highlight_style(SELECTED_STYLE)
    .highlight_spacing(HighlightSpacing::Always);

    // Rendering with state lets ratatui scroll the selection into view.
    let mut state = TableState::new().with_selected(Some(selected_row));
    f.render_stateful_widget(table, inner, &mut state);
}

fn draw_schedule(f: &mut Frame, app: &App, area: Rect) {
    let block = panel(
        date_title(app, "Schedule"),
        " \u{25C4} h/Left  |  l/Right \u{25BA}  |  j/k \u{2195} ",
    );
    let inner = block.inner(area);
    f.render_widget(block, area);

    let Some(schedule) = &app.schedule else {
        return placeholder(f, inner, "Loading\u{2026}", Style::new());
    };

    let mut lines: Vec<Line> = Vec::new();
    let mut game_idx = 0usize;
    let mut selected_line = 0usize;

    for day in &schedule.game_week {
        lines.push(Line::styled(
            format!("\u{2500} {} {} \u{2500}", day.day_abbrev, day.date),
            HEADING,
        ));
        for game in &day.games {
            let selected = game_idx == app.schedule_scroll;
            if selected {
                selected_line = lines.len();
            }
            let city = |team: &crate::api::models::ScheduleTeam| {
                let place = team
                    .place_name
                    .as_ref()
                    .map(|n| n.default.as_str())
                    .unwrap_or("");
                format!("{} {}", place, team.abbrev)
                    .trim_start()
                    .to_string()
            };
            let team_style = |abbrev: &str| {
                if app.is_favorite_team(abbrev) {
                    FAVORITE
                } else {
                    Style::new()
                }
            };

            lines.push(
                Line::from(vec![
                    Span::raw(" "),
                    Span::raw(if selected { "\u{25B8}" } else { " " }),
                    Span::styled(
                        format!("{:>8}  ", start_time(Some(&game.start_time_utc))),
                        MUTED,
                    ),
                    Span::styled(city(&game.away_team), team_style(&game.away_team.abbrev)),
                    Span::raw(" @ "),
                    Span::styled(city(&game.home_team), team_style(&game.home_team.abbrev)),
                ])
                .style(if selected {
                    Style::new().bg(SELECTED_BG)
                } else {
                    Style::new()
                }),
            );
            game_idx += 1;
        }
        lines.push(Line::raw(""));
    }

    if game_idx == 0 {
        return placeholder(f, inner, "No games.", MUTED);
    }

    let offset = scroll_offset(selected_line, inner.height as usize, lines.len());
    f.render_widget(Paragraph::new(lines).scroll((offset as u16, 0)), inner);
}

fn draw_leaders(f: &mut Frame, app: &App, area: Rect) {
    let block = panel(
        format!(" Stat Leaders [{}] ", app.leader_category.as_str()),
        " \u{25C4} h  |  l \u{25BA}  |  j/k \u{2195} ",
    );
    let inner = block.inner(area);
    f.render_widget(block, area);

    let entries = app.current_leaders();
    if entries.is_empty() {
        let text = if app.last_updated.is_some() {
            "No leaders for this category."
        } else {
            "Loading\u{2026}"
        };
        return placeholder(f, inner, text, MUTED);
    }

    let rows: Vec<Row> = entries
        .iter()
        .enumerate()
        .map(|(i, e)| {
            Row::new(vec![
                (i + 1).to_string(),
                format!("{} {}", text(&e.first_name), text(&e.last_name))
                    .trim()
                    .to_string(),
                e.position.clone().unwrap_or_default(),
                e.team_abbrev.clone().unwrap_or_default(),
                e.value
                    .map(|v| app.leader_category.format_value(v))
                    .unwrap_or_default(),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(3),
            Constraint::Min(20),
            Constraint::Length(4),
            Constraint::Length(5),
            Constraint::Length(8),
        ],
    )
    .header(header_row(&["#", "Player", "Pos", "Team", "Value"]))
    .row_highlight_style(SELECTED_STYLE)
    .highlight_spacing(HighlightSpacing::Always);

    let mut state = TableState::new().with_selected(Some(app.leaders_scroll));
    f.render_stateful_widget(table, inner, &mut state);
}

fn draw_status(f: &mut Frame, app: &App, area: Rect) {
    let mut spans = vec![Span::raw("  ")];
    if let Some(updated) = &app.last_updated {
        spans.push(Span::styled(
            format!("Updated: {}  ", updated.format("%H:%M:%S")),
            MUTED,
        ));
    }
    if app.loading {
        spans.push(Span::styled(
            "\u{27F3} Loading\u{2026}  ",
            Style::new().fg(Color::Yellow),
        ));
    }
    if let Some(team) = &app.favorite_team {
        spans.push(Span::styled(format!("\u{2605} {team}  "), FAVORITE));
    }
    // Errors used to go to stderr, which is the stream this screen is drawn
    // on; showing them here keeps the display intact.
    if let Some(error) = &app.error {
        spans.push(Span::styled(
            format!("\u{26A0} {error}  "),
            Style::new().fg(Color::Red),
        ));
    }
    spans.push(Span::styled("[q]uit [r]efresh [1-4]tabs", MUTED));
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn draw_boxscore_overlay(f: &mut Frame, app: &App, area: Rect) {
    let Some(boxscore) = &app.boxscore else {
        let r = centered_rect(40, 20, area);
        f.render_widget(Clear, r);
        f.render_widget(
            Block::default()
                .title(" Loading\u{2026} ")
                .borders(Borders::ALL)
                .border_style(Style::new().fg(Color::Yellow)),
            r,
        );
        return;
    };

    let overlay = centered_rect(80, 80, area);
    f.render_widget(Clear, overlay);

    let (away, home) = (&boxscore.away_team, &boxscore.home_team);
    let team_name = |team: &crate::api::models::BoxscoreTeam| {
        team.name
            .as_ref()
            .map(|n| n.default.as_str())
            .unwrap_or(&team.abbrev)
            .to_string()
    };

    let block = Block::default()
        .title(format!(
            " {} {} - {} {} ",
            team_name(away),
            away.score.unwrap_or(0),
            home.score.unwrap_or(0),
            team_name(home),
        ))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::Yellow));
    let inner = block.inner(overlay);
    f.render_widget(block, overlay);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(3),
            Constraint::Length(1),
        ])
        .split(inner);

    let scoring = boxscore.summary.as_ref().and_then(|s| s.scoring.as_ref());

    let mut period_lines = vec![Line::from(vec![
        Span::raw("        "),
        Span::styled("1st   2nd   3rd   OT    Total", Style::new().bold()),
    ])];

    if let Some(periods) = scoring {
        let (mut away_goals, mut home_goals) = ([0u32; 4], [0u32; 4]);
        for period in periods {
            let idx = (period.period_descriptor.number as usize).clamp(1, 4) - 1;
            for goal in &period.goals {
                if goal.team_abbrev.default == away.abbrev {
                    away_goals[idx] += 1;
                } else {
                    home_goals[idx] += 1;
                }
            }
        }
        let row = |abbrev: &str, goals: &[u32; 4]| {
            Line::from(vec![
                Span::styled(format!("{abbrev:<8}"), Style::new().bold()),
                Span::raw(format!(
                    "{:<5} {:<5} {:<5} {:<5} {}",
                    goals[0],
                    goals[1],
                    goals[2],
                    goals[3],
                    goals.iter().sum::<u32>()
                )),
            ])
        };
        period_lines.push(row(&away.abbrev, &away_goals));
        period_lines.push(row(&home.abbrev, &home_goals));
    }
    f.render_widget(Paragraph::new(period_lines), chunks[0]);

    let mut goal_lines = vec![Line::styled("   Goals:", Style::new().bold())];
    if let Some(periods) = scoring {
        for period in periods {
            // Built per goal rather than leaked: this runs on every frame.
            let period_label = period.period_descriptor.label();
            for goal in &period.goals {
                let scorer = format!(
                    "{} {}{}",
                    initial(&goal.first_name),
                    text(&goal.last_name),
                    goal.goals_to_date
                        .map(|n| format!(" ({n})"))
                        .unwrap_or_default()
                );
                let assists = goal
                    .assists
                    .iter()
                    .map(|a| {
                        format!(
                            "{} {}{}",
                            initial(&a.first_name),
                            text(&a.last_name),
                            a.assists_to_date
                                .map(|n| format!(" ({n})"))
                                .unwrap_or_default()
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");

                let mut spans = vec![
                    Span::styled(format!("   {period_label:<5} "), MUTED),
                    Span::styled(format!("{:<5} ", goal.time_in_period), MUTED),
                    Span::styled(format!("{} - ", goal.team_abbrev.default), HEADING),
                    Span::styled(scorer, Style::new().bold()),
                ];
                if let Some(strength) = goal.strength.as_deref().filter(|s| *s != "ev") {
                    spans.push(Span::styled(
                        format!(" {}", strength.to_uppercase()),
                        Style::new().fg(Color::Yellow),
                    ));
                }
                spans.push(Span::styled(
                    if assists.is_empty() {
                        "  [unassisted]".to_string()
                    } else {
                        format!("  [{assists}]")
                    },
                    MUTED,
                ));
                goal_lines.push(Line::from(spans));
            }
        }
    }
    if goal_lines.len() == 1 {
        goal_lines.push(Line::styled("   No goals.", MUTED));
    }

    f.render_widget(Paragraph::new(goal_lines), chunks[1]);
    f.render_widget(
        Paragraph::new(Line::styled("[Esc] Close", MUTED)).alignment(Alignment::Center),
        chunks[2],
    );
}

fn initial(name: &Option<crate::api::models::NameField>) -> String {
    name.as_ref()
        .and_then(|n| n.default.chars().next())
        .map(|c| format!("{c}."))
        .unwrap_or_default()
}

/// The value of an optional `{ "default": ... }` field, or "".
fn text(name: &Option<crate::api::models::NameField>) -> &str {
    name.as_ref().map(|n| n.default.as_str()).unwrap_or("")
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

#[cfg(test)]
mod tests {
    use super::scroll_offset;

    #[test]
    fn short_lists_never_scroll() {
        assert_eq!(scroll_offset(0, 10, 5), 0);
        assert_eq!(scroll_offset(4, 10, 5), 0);
    }

    #[test]
    fn selection_stays_visible_in_long_lists() {
        // 20 rows in a 5-row viewport.
        assert_eq!(scroll_offset(0, 5, 20), 0);
        assert_eq!(scroll_offset(4, 5, 20), 0, "still on the first screen");
        assert_eq!(scroll_offset(5, 5, 20), 1);
        assert_eq!(scroll_offset(19, 5, 20), 15, "clamped to the last screen");
    }

    #[test]
    fn zero_height_is_handled() {
        assert_eq!(scroll_offset(3, 0, 20), 0);
    }
}

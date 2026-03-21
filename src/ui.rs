use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, HighlightSpacing, Paragraph, Row, Table, Tabs, Wrap},
    Frame,
};

use crate::app::{App, StandingsFilter, Tab};

const SELECTED_STYLE: Style = Style::new().bg(Color::DarkGray).fg(Color::White).add_modifier(Modifier::BOLD);

pub fn draw(f: &mut Frame, app: &App) {
    let size = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(size);

    draw_tabs(f, app, chunks[0]);
    draw_content(f, app, chunks[1]);
    draw_status(f, app, chunks[2]);

    if app.show_boxscore {
        draw_boxscore_overlay(f, app, size);
    }
}

fn draw_tabs(f: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<&str> = vec!["[1] Scores", "[2] Standings", "[3] Schedule", "[4] Leaders"];
    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .title(" NHL Dashboard ")
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .select(app.active_tab as usize)
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    f.render_widget(tabs, area);
}

fn draw_content(f: &mut Frame, app: &App, area: Rect) {
    match app.active_tab {
        Tab::Scores => draw_scores(f, app, area),
        Tab::Standings => draw_standings(f, app, area),
        Tab::Schedule => draw_schedule(f, app, area),
        Tab::Leaders => draw_leaders(f, app, area),
    }
}

fn draw_scores(f: &mut Frame, app: &App, area: Rect) {
    let label = if app.is_today() {
        format!(" {} (Today) ", app.date_str())
    } else {
        format!(" {} ", app.date_str())
    };

    let block = Block::default()
        .title(format!(" Scores {} ", label))
        .title_bottom(" \u{25C4} h/Left  |  l/Right \u{25BA}  |  j/k \u{2195}  |  Enter: details ")
        .borders(Borders::ALL);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let scores = match &app.scores {
        Some(s) => s,
        None => {
            f.render_widget(Paragraph::new("Loading...").alignment(Alignment::Center), inner);
            return;
        }
    };

    if scores.games.is_empty() {
        f.render_widget(
            Paragraph::new("No games.").alignment(Alignment::Center)
                .style(Style::default().fg(Color::DarkGray)),
            inner,
        );
        return;
    }

    let fav = app.favorite_team.as_deref().unwrap_or("");
    let lines: Vec<Line> = scores.games.iter().enumerate().map(|(i, game)| {
        let sel = i == app.scores_scroll;
        let is_fav = game.away_team.abbrev == fav || game.home_team.abbrev == fav;

        let state = match game.game_state.as_str() {
            "LIVE" => {
                let p = game.period.unwrap_or(0);
                let c = game.clock.as_ref().and_then(|c| c.time_remaining.clone()).unwrap_or_default();
                (format!("LIVE {}P {}", p, c), Style::default().fg(Color::Green))
            }
            "FINAL" | "OFF" => {
                let ot = game.game_outcome.as_ref()
                    .and_then(|o| o.last_period_type.clone()).unwrap_or_default();
                let label = if ot == "OT" { "FINAL/OT" } else if ot == "SO" { "FINAL/SO" } else { "FINAL" };
                (label.to_string(), Style::default().fg(Color::DarkGray))
            }
            "FUT" | "PRE" => {
                let t = game.start_time_utc.as_ref().and_then(|t|
                    chrono::DateTime::parse_from_rfc3339(t).ok()
                        .map(|d| d.with_timezone(&chrono::Local).format("%I:%M %p").to_string())
                ).unwrap_or("TBD".into());
                (t, Style::default().fg(Color::Blue))
            }
            s => (s.to_string(), Style::default().fg(Color::White)),
        };

        let a_s = game.away_team.score.unwrap_or(0);
        let h_s = game.home_team.score.unwrap_or(0);
        let a_style = if is_fav && game.away_team.abbrev == fav {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else { Style::default() };
        let h_style = if is_fav && game.home_team.abbrev == fav {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else { Style::default() };

        let cursor = if sel { "\u{25B8} " } else { "  " };

        let mut spans = vec![
            Span::styled(cursor, if sel { SELECTED_STYLE } else { Style::default() }),
            Span::styled(format!("{:>3} ", game.away_team.abbrev), if sel { a_style.bg(Color::DarkGray) } else { a_style }),
            Span::styled(format!("{:>2}", a_s), if sel { Style::default().add_modifier(Modifier::BOLD).bg(Color::DarkGray) } else { Style::default().add_modifier(Modifier::BOLD) }),
            Span::styled(" - ", if sel { SELECTED_STYLE } else { Style::default() }),
            Span::styled(format!("{:<2}", h_s), if sel { Style::default().add_modifier(Modifier::BOLD).bg(Color::DarkGray) } else { Style::default().add_modifier(Modifier::BOLD) }),
            Span::styled(format!("{:<3}", game.home_team.abbrev), if sel { h_style.bg(Color::DarkGray) } else { h_style }),
            Span::styled("  ", if sel { SELECTED_STYLE } else { Style::default() }),
            Span::styled(state.0, if sel { state.1.bg(Color::DarkGray) } else { state.1 }),
        ];
        if sel {
            spans.push(Span::styled("  \u{21B5} Enter", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
        }
        Line::from(spans)
    }).collect();

    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

fn draw_standings(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(format!(" Standings [{}] ", app.standings_filter.as_str()))
        .title_bottom(" \u{25C4} h  |  l \u{25BA}  |  j/k \u{2195} ")
        .borders(Borders::ALL);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let filtered = app.filtered_standings();
    if filtered.is_empty() {
        f.render_widget(Paragraph::new("Loading...").alignment(Alignment::Center), inner);
        return;
    }

    let mut current_div = String::new();
    let mut rows = Vec::new();
    let mut data_row_idx = 0usize;

    for (i, s) in filtered.iter().enumerate() {
        let is_fav = app.is_favorite_team(&s.team_abbrev.default);

        if app.standings_filter == StandingsFilter::Division && s.division_name != current_div {
            current_div = s.division_name.clone();
            rows.push(Row::new(vec![Cell::from(format!("\u{2500} {} \u{2500}", current_div))
                .style(Style::default().fg(Color::Cyan))]));
        }
        if app.standings_filter == StandingsFilter::Conference && s.conference_name != current_div {
            current_div = s.conference_name.clone();
            rows.push(Row::new(vec![Cell::from(format!("\u{2500} {} \u{2500}", current_div))
                .style(Style::default().fg(Color::Cyan))]));
        }

        let is_selected = data_row_idx == app.standings_scroll;
        let seq = match app.standings_filter {
            StandingsFilter::League => s.league_sequence.unwrap_or(i as u32 + 1),
            StandingsFilter::Conference => s.conference_sequence.unwrap_or(i as u32 + 1),
            StandingsFilter::Division => s.division_sequence.unwrap_or(i as u32 + 1),
        };

        let streak = format!("{}{}", s.streak_code.as_deref().unwrap_or(""), s.streak_count.unwrap_or(0));
        let style = if is_selected {
            SELECTED_STYLE
        } else if is_fav {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else { Style::default() };

        rows.push(Row::new(vec![
            Cell::from(format!("{}", seq)),
            Cell::from(format!("{} ({})", s.team_name.default, s.team_abbrev.default)),
            Cell::from(format!("{}", s.games_played)),
            Cell::from(format!("{}", s.wins)),
            Cell::from(format!("{}", s.losses)),
            Cell::from(format!("{}", s.ot_losses)),
            Cell::from(format!("{}", s.points)),
            Cell::from(format!("{}", s.goal_for)),
            Cell::from(format!("{}", s.goal_against)),
            Cell::from(format!("{}{}", if s.goal_differential > 0 { "+" } else { "" }, s.goal_differential)),
            Cell::from(streak),
        ]).style(style));
        data_row_idx += 1;
    }

    let header = Row::new(vec![
        Cell::from("#").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Team").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("GP").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("W").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("L").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("OT").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("PTS").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("GF").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("GA").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("+/-").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("STRK").style(Style::default().add_modifier(Modifier::BOLD)),
    ]);

    let table = Table::new(rows, [
        Constraint::Length(3), Constraint::Min(20), Constraint::Length(4),
        Constraint::Length(4), Constraint::Length(4), Constraint::Length(4),
        Constraint::Length(5), Constraint::Length(5), Constraint::Length(5),
        Constraint::Length(5), Constraint::Length(6),
    ]).header(header).highlight_spacing(HighlightSpacing::Always);
    f.render_widget(table, inner);
}

fn draw_schedule(f: &mut Frame, app: &App, area: Rect) {
    let label = if app.is_today() {
        format!(" {} (Today) ", app.date_str())
    } else {
        format!(" {} ", app.date_str())
    };

    let block = Block::default()
        .title(format!(" Schedule {} ", label))
        .title_bottom(" \u{25C4} h/Left  |  l/Right \u{25BA} ")
        .borders(Borders::ALL);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let schedule = match &app.schedule {
        Some(s) => s,
        None => {
            f.render_widget(Paragraph::new("Loading...").alignment(Alignment::Center), inner);
            return;
        }
    };

    let fav = app.favorite_team.as_deref().unwrap_or("");
    let mut lines: Vec<Line> = Vec::new();
    let mut game_idx = 0usize;

    for day in &schedule.game_week {
        lines.push(Line::from(Span::styled(
            format!("\u{2500} {} {} \u{2500}", day.day_abbrev, day.date),
            Style::default().fg(Color::Cyan),
        )));
        for game in &day.games {
            let sel = game_idx == app.schedule_scroll;
            let is_fav = game.away_team.abbrev == fav || game.home_team.abbrev == fav;
            let a_city = game.away_team.place_name.as_ref().map(|n| n.default.as_str()).unwrap_or("");
            let h_city = game.home_team.place_name.as_ref().map(|n| n.default.as_str()).unwrap_or("");
            let time = chrono::DateTime::parse_from_rfc3339(&game.start_time_utc)
                .map(|d| d.with_timezone(&chrono::Local).format("%I:%M %p").to_string())
                .unwrap_or("TBD".into());

            let (time_style, a_style, h_style) = if sel {
                (Style::default().bg(Color::DarkGray), Style::default().bg(Color::DarkGray), Style::default().bg(Color::DarkGray))
            } else if is_fav {
                (Style::default().fg(Color::DarkGray), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            } else {
                (Style::default().fg(Color::DarkGray), Style::default(), Style::default())
            };

            let cursor = if sel { Span::styled("\u{25B8}", SELECTED_STYLE) } else { Span::raw(" ") };

            lines.push(Line::from(vec![
                Span::raw(" "),
                cursor,
                Span::styled(format!("{:>8}  ", time), time_style),
                Span::styled(format!("{} {}", a_city, game.away_team.abbrev), a_style),
                Span::styled(" @ ", if sel { SELECTED_STYLE } else { Style::default() }),
                Span::styled(format!("{} {}", h_city, game.home_team.abbrev), h_style),
            ]));
            game_idx += 1;
        }
        lines.push(Line::raw(""));
    }

    if lines.is_empty() {
        lines.push(Line::from(Span::raw("No games.").style(Style::default().fg(Color::DarkGray))));
    }
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

fn draw_leaders(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(format!(" Stat Leaders [{}] ", app.leader_category.as_str()))
        .title_bottom(" \u{25C4} h  |  l \u{25BA}  |  j/k \u{2195} ")
        .borders(Borders::ALL);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let entries = app.current_leaders();
    if entries.is_empty() {
        f.render_widget(Paragraph::new("Loading...").alignment(Alignment::Center), inner);
        return;
    }

    let header = Row::new(vec![
        Cell::from("#").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Player").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Pos").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Team").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Value").style(Style::default().add_modifier(Modifier::BOLD)),
    ]);

    let rows: Vec<Row> = entries.iter().enumerate().map(|(i, e)| {
        let sel = i == app.leaders_scroll;
        let first = e.first_name.as_ref().map(|n| n.default.as_str()).unwrap_or("");
        let last = e.last_name.as_ref().map(|n| n.default.as_str()).unwrap_or("");
        let val = e.value.as_ref().map(|v| v.to_string()).unwrap_or_default();
        let style = if sel { SELECTED_STYLE } else { Style::default() };
        Row::new(vec![
            Cell::from(format!("{}", i + 1)),
            Cell::from(format!("{} {}", first, last)),
            Cell::from(e.position.as_deref().unwrap_or("").to_string()),
            Cell::from(e.team_abbrev.as_deref().unwrap_or("").to_string()),
            Cell::from(val),
        ]).style(style)
    }).collect();

    let table = Table::new(rows, [
        Constraint::Length(3), Constraint::Min(20), Constraint::Length(4),
        Constraint::Length(5), Constraint::Length(8),
    ]).header(header).highlight_spacing(HighlightSpacing::Always);
    f.render_widget(table, inner);
}

fn draw_status(f: &mut Frame, app: &App, area: Rect) {
    let mut spans = vec![Span::raw("  ")];
    if let Some(u) = &app.last_updated {
        spans.push(Span::styled(format!("Updated: {}  ", u.format("%H:%M:%S")), Style::default().fg(Color::DarkGray)));
    }
    if app.loading {
        spans.push(Span::styled("\u{27F3} Loading...  ", Style::default().fg(Color::Yellow)));
    }
    if let Some(t) = &app.favorite_team {
        spans.push(Span::styled(format!("\u{2605} {}  ", t), Style::default().fg(Color::Yellow)));
    }
    spans.push(Span::styled("[q]uit [r]efresh [1-4]tabs", Style::default().fg(Color::DarkGray)));
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn draw_boxscore_overlay(f: &mut Frame, app: &App, area: Rect) {
    let boxscore = match &app.boxscore {
        Some(b) => b,
        None => {
            let r = centered_rect(40, 20, area);
            f.render_widget(Block::default().title(" Loading... ").borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)), r);
            return;
        }
    };

    let overlay = centered_rect(80, 80, area);
    f.render_widget(ratatui::widgets::Clear, overlay);

    let away = &boxscore.away_team;
    let home = &boxscore.home_team;
    let title = format!(
        " {} {} - {} {} ",
        away.name.as_ref().map(|n| n.default.as_str()).unwrap_or(&away.abbrev),
        away.score.unwrap_or(0),
        home.score.unwrap_or(0),
        home.name.as_ref().map(|n| n.default.as_str()).unwrap_or(&home.abbrev),
    );

    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    let inner = block.inner(overlay);
    f.render_widget(block, overlay);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(5), Constraint::Length(1)])
        .split(inner);

    let mut period_lines = vec![
        Line::from(vec![
            Span::styled("         ", Style::default()),
            Span::styled("1st   2nd   3rd   OT    Total",
                Style::default().add_modifier(Modifier::BOLD)),
        ]),
    ];

    let (scoring, away_abbr, home_abbr) = match &boxscore.summary {
        Some(summary) => {
            let scoring = summary.scoring.as_ref();
            (scoring, away.abbrev.as_str(), home.abbrev.as_str())
        }
        None => (None, away.abbrev.as_str(), home.abbrev.as_str()),
    };

    if let Some(scoring_periods) = scoring {
        let mut away_p = vec![0u32; 4];
        let mut home_p = vec![0u32; 4];
        for period in scoring_periods {
            let idx = (period.period_descriptor.number as usize).saturating_sub(1).min(3);
            for goal in &period.goals {
                if goal.team_abbrev.default == away_abbr {
                    away_p[idx] += 1;
                } else {
                    home_p[idx] += 1;
                }
            }
        }
        let fmt = |p: &[u32]| format!("{:<5} {:<5} {:<5} {:<5} {}", p[0], p[1], p[2], p[3], p.iter().sum::<u32>());
        period_lines.push(Line::from(vec![
            Span::styled(format!("{:<8}", away_abbr), Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(fmt(&away_p)),
        ]));
        period_lines.push(Line::from(vec![
            Span::styled(format!("{:<8}", home_abbr), Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(fmt(&home_p)),
        ]));
    }
    f.render_widget(Paragraph::new(period_lines), chunks[0]);

    let mut goal_lines: Vec<Line> = vec![
        Line::from(Span::styled("   Goals:", Style::default().add_modifier(Modifier::BOLD)))
    ];

    if let Some(scoring_periods) = scoring {
        for period in scoring_periods {
            let pl = match period.period_descriptor.number {
                1 => "1st", 2 => "2nd", 3 => "3rd",
                n => { let s = format!("OT{}", n-3); Box::leak(s.into_boxed_str()) }
            };
            for goal in &period.goals {
                let first_initial = goal.first_name.as_ref()
                    .and_then(|n| n.default.chars().next())
                    .map(|c| format!("{}.", c))
                    .unwrap_or_default();
                let last = goal.last_name.as_ref().map(|n| n.default.as_str()).unwrap_or("");
                let goal_num = goal.goals_to_date
                    .map(|n| format!(" ({})", n))
                    .unwrap_or_default();

                let assists = goal.assists.iter()
                    .map(|a| {
                        let af = a.first_name.as_ref()
                            .and_then(|n| n.default.chars().next())
                            .map(|c| format!("{}.", c))
                            .unwrap_or_default();
                        let al = a.last_name.as_ref().map(|n| n.default.as_str()).unwrap_or("");
                        let ast_num = a.assists_to_date
                            .map(|n| format!(" ({})", n))
                            .unwrap_or_default();
                        format!("{} {}{}", af, al, ast_num)
                    })
                    .collect::<Vec<_>>().join(", ");

                goal_lines.push(Line::from(vec![
                    Span::styled(format!("   {:<5} ", pl), Style::default().fg(Color::DarkGray)),
                    Span::styled(format!("{:<5} ", goal.time_in_period), Style::default().fg(Color::DarkGray)),
                    Span::styled(format!("{} - ", goal.team_abbrev.default), Style::default().fg(Color::Cyan)),
                    Span::styled(format!("{} {}{}", first_initial, last, goal_num), Style::default().add_modifier(Modifier::BOLD)),
                    if let Some(strength) = &goal.strength {
                        if strength != "ev" {
                            Span::styled(format!(" {}", strength.to_uppercase()), Style::default().fg(Color::Yellow))
                        } else { Span::raw("") }
                    } else { Span::raw("") },
                    if !assists.is_empty() {
                        Span::styled(format!("  [{}]", assists), Style::default().fg(Color::DarkGray))
                    } else { Span::styled("  [unassisted]", Style::default().fg(Color::DarkGray)) },
                ]));
            }
        }
    }

    if goal_lines.len() == 1 {
        goal_lines.push(Line::from(Span::raw("No goals.").style(Style::default().fg(Color::DarkGray))));
    }

    f.render_widget(Paragraph::new(goal_lines).wrap(Wrap { trim: false }), chunks[1]);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled("[Esc] Close", Style::default().fg(Color::DarkGray))))
            .alignment(Alignment::Center),
        chunks[2],
    );
}

fn centered_rect(px: u16, py: u16, r: Rect) -> Rect {
    let v = Layout::default().direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - py) / 2),
            Constraint::Percentage(py),
            Constraint::Percentage((100 - py) / 2),
        ]).split(r);
    Layout::default().direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - px) / 2),
            Constraint::Percentage(px),
            Constraint::Percentage((100 - px) / 2),
        ]).split(v[1])[1]
}

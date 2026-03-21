mod action;
mod api;
mod app;
mod errors;
mod tui;
mod ui;

use clap::Parser;
use color_eyre::eyre::Result;

#[derive(Parser)]
#[command(name = "nhl-tui", version, about = "NHL scores dashboard for the terminal")]
struct Cli {
    /// Favorite team abbreviation (e.g., TOR, EDM, BOS)
    #[arg(short, long)]
    team: Option<String>,

    /// Starting tab (1=Scores, 2=Standings, 3=Schedule, 4=Leaders)
    #[arg(short, long, default_value = "1")]
    tab: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();
    let start_tab = (cli.tab.saturating_sub(1)).min(3);

    let mut app = app::App::new(cli.team, start_tab);
    let mut tui_inst = tui::Tui::new()?;

    tui_inst.run(&mut app).await?;

    Ok(())
}

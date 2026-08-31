mod action;
mod api;
mod app;
mod tui;
mod ui;

use clap::Parser;
use color_eyre::eyre::Result;

#[derive(Parser)]
#[command(
    name = "nhl-tui",
    version,
    about = "NHL scores dashboard for the terminal"
)]
struct Cli {
    /// Favorite team abbreviation (e.g., TOR, EDM, BOS)
    #[arg(short, long)]
    team: Option<String>,

    /// Starting tab (1=Scores, 2=Standings, 3=Schedule, 4=Leaders)
    ///
    /// No short form: `-t` belongs to `--team`.
    #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u8).range(1..=4))]
    tab: u8,
}

/// Restores the terminal before a panic or error report is printed, so the
/// message lands on a usable screen instead of the alternate one.
fn install_hooks() -> Result<()> {
    let (panic_hook, eyre_hook) = color_eyre::config::HookBuilder::default().into_hooks();

    let eyre_hook = eyre_hook.into_eyre_hook();
    color_eyre::eyre::set_hook(Box::new(move |error| {
        let _ = tui::restore();
        eyre_hook(error)
    }))?;

    let panic_hook = panic_hook.into_panic_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = tui::restore();
        panic_hook(info);
    }));

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    install_hooks()?;

    let mut app = app::App::new(cli.team, usize::from(cli.tab - 1));
    let mut tui = tui::Tui::new()?;

    tui.run(&mut app).await
}

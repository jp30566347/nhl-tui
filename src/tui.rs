use std::io::Write;
use std::time::Duration;

use color_eyre::eyre::Result;
use crossterm::event::{Event as CrosstermEvent, EventStream, KeyEventKind};
use futures::{FutureExt, StreamExt};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::action::Action;
use crate::app::App;
use crate::ui;

const REFRESH_INTERVAL: Duration = Duration::from_secs(30);

type Backend = ratatui::backend::CrosstermBackend<std::io::Stderr>;

pub struct Tui {
    terminal: ratatui::Terminal<Backend>,
    task: tokio::task::JoinHandle<()>,
    cancellation_token: CancellationToken,
    action_rx: mpsc::UnboundedReceiver<Action>,
    action_tx: mpsc::UnboundedSender<Action>,
}

/// Puts the terminal back the way we found it.
///
/// Safe to call more than once, and callable from a panic hook, which is why
/// it is a free function rather than a method on `Tui`.
pub fn restore() -> Result<()> {
    crossterm::execute!(
        std::io::stderr(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::cursor::Show
    )?;
    crossterm::terminal::disable_raw_mode()?;
    Ok(())
}

impl Tui {
    pub fn new() -> Result<Self> {
        let terminal =
            ratatui::Terminal::new(ratatui::backend::CrosstermBackend::new(std::io::stderr()))?;
        let (action_tx, action_rx) = mpsc::unbounded_channel();
        Ok(Self {
            terminal,
            task: tokio::spawn(async {}),
            cancellation_token: CancellationToken::new(),
            action_rx,
            action_tx,
        })
    }

    fn start(&mut self) {
        self.cancel();
        self.cancellation_token = CancellationToken::new();
        let cancellation_token = self.cancellation_token.clone();
        let action_tx = self.action_tx.clone();

        self.task = tokio::spawn(async move {
            let mut reader = EventStream::new();
            let mut refresh = tokio::time::interval(REFRESH_INTERVAL);
            // The first tick of an interval completes immediately; the initial
            // fetch is issued by `run`, so skip it.
            refresh.tick().await;

            loop {
                tokio::select! {
                    _ = cancellation_token.cancelled() => break,
                    maybe_event = reader.next().fuse() => {
                        match maybe_event {
                            Some(Ok(CrosstermEvent::Key(key)))
                                if key.kind == KeyEventKind::Press =>
                            {
                                if action_tx.send(Action::Key(key)).is_err() {
                                    break;
                                }
                            }
                            // A resize invalidates the whole screen.
                            Some(Ok(CrosstermEvent::Resize(_, _))) => {
                                if action_tx.send(Action::Render).is_err() {
                                    break;
                                }
                            }
                            Some(Ok(_)) => {}
                            // stdin closed or broke: nothing more will arrive.
                            Some(Err(_)) | None => break,
                        }
                    }
                    _ = refresh.tick() => {
                        if action_tx.send(Action::Refresh).is_err() {
                            break;
                        }
                    }
                }
            }
        });
    }

    fn cancel(&self) {
        self.cancellation_token.cancel();
    }

    fn enter(&mut self) -> Result<()> {
        crossterm::terminal::enable_raw_mode()?;
        // Mouse capture is deliberately not enabled: nothing here handles
        // mouse events, and turning it on breaks click-to-select in the
        // host terminal.
        crossterm::execute!(
            std::io::stderr(),
            crossterm::terminal::EnterAlternateScreen,
            crossterm::cursor::Hide
        )?;
        self.terminal.clear()?;
        Ok(())
    }

    pub async fn run(&mut self, app: &mut App) -> Result<()> {
        self.enter()?;
        self.start();

        // Restore the terminal even if the loop fails, so an error is
        // readable instead of being printed into the alternate screen.
        let result = self.event_loop(app).await;
        self.cancel();
        restore()?;
        result
    }

    async fn event_loop(&mut self, app: &mut App) -> Result<()> {
        app.spawn_fetch(self.action_tx.clone(), true);
        self.draw(app)?;

        while let Some(action) = self.action_rx.recv().await {
            match action {
                Action::Render => {}
                Action::Refresh => app.spawn_fetch(self.action_tx.clone(), false),
                Action::ForceRefresh => app.spawn_fetch(self.action_tx.clone(), true),
                Action::Fetched(fetched) => {
                    if app.apply_fetch(*fetched) {
                        bell();
                    }
                }
                Action::Key(key) => {
                    if let Some(next) = app.handle_key(key) {
                        // The channel is only closed once we drop it.
                        let _ = self.action_tx.send(next);
                    }
                }
            }

            if app.should_quit {
                break;
            }
            // Every action either changed state or asked for a redraw.
            self.draw(app)?;
        }
        Ok(())
    }

    fn draw(&mut self, app: &App) -> Result<()> {
        self.terminal.draw(|frame| ui::draw(frame, app))?;
        Ok(())
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        self.cancel();
        self.task.abort();
    }
}

/// Rings the terminal bell. Written straight to the terminal because it is
/// not something that can live in a ratatui cell buffer.
fn bell() {
    let mut stderr = std::io::stderr();
    let _ = stderr.write_all(b"\x07");
    let _ = stderr.flush();
}

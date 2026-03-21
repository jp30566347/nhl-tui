use std::time::Duration;

use color_eyre::eyre::Result;
use crossterm::event::{Event as CrosstermEvent, EventStream, KeyEventKind};
use futures::{FutureExt, StreamExt};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::action::Action;
use crate::app::App;
use crate::ui;

pub struct Tui {
    pub terminal: ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stderr>>,
    task: tokio::task::JoinHandle<()>,
    cancellation_token: CancellationToken,
    action_rx: mpsc::UnboundedReceiver<Action>,
    action_tx: mpsc::UnboundedSender<Action>,
    frame_rate: f64,
    tick_rate: f64,
    refresh_interval: Duration,
}

impl Tui {
    pub fn new() -> Result<Self> {
        let terminal = ratatui::Terminal::new(ratatui::backend::CrosstermBackend::new(
            std::io::stderr(),
        ))?;
        let (action_tx, action_rx) = mpsc::unbounded_channel();
        Ok(Self {
            terminal,
            task: tokio::spawn(async {}),
            cancellation_token: CancellationToken::new(),
            action_rx,
            action_tx,
            frame_rate: 30.0,
            tick_rate: 4.0,
            refresh_interval: Duration::from_secs(30),
        })
    }

    pub fn start(&mut self) {
        let tick_delay = Duration::from_secs_f64(1.0 / self.tick_rate);
        let render_delay = Duration::from_secs_f64(1.0 / self.frame_rate);
        let refresh_delay = self.refresh_interval;
        self.cancel();
        self.cancellation_token = CancellationToken::new();
        let cancellation_token = self.cancellation_token.clone();
        let action_tx = self.action_tx.clone();

        self.task = tokio::spawn(async move {
            let mut reader = EventStream::new();
            let mut tick_interval = tokio::time::interval(tick_delay);
            let mut render_interval = tokio::time::interval(render_delay);
            let mut refresh_interval = tokio::time::interval(refresh_delay);

            loop {
                let tick_delay = tick_interval.tick();
                let render_delay = render_interval.tick();
                let refresh_tick = refresh_interval.tick();
                let crossterm_event = reader.next().fuse();

                tokio::select! {
                    _ = cancellation_token.cancelled() => {
                        break;
                    }
                    maybe_event = crossterm_event => {
                        if let Some(Ok(CrosstermEvent::Key(key))) = maybe_event {
                            if key.kind == KeyEventKind::Press {
                                let _ = action_tx.send(Action::Key(key));
                            }
                        }
                    },
                    _ = tick_delay => {
                        let _ = action_tx.send(Action::Tick);
                    },
                    _ = render_delay => {
                        let _ = action_tx.send(Action::Render);
                    },
                    _ = refresh_tick => {
                        let _ = action_tx.send(Action::Tick);
                    },
                }
            }
        });
    }

    fn cancel(&self) {
        self.cancellation_token.cancel();
    }

    pub fn enter(&mut self) -> Result<()> {
        crossterm::terminal::enable_raw_mode()?;
        crossterm::execute!(
            std::io::stderr(),
            crossterm::terminal::EnterAlternateScreen,
            crossterm::event::EnableMouseCapture
        )?;
        self.terminal.clear()?;
        Ok(())
    }

    pub fn exit(&mut self) -> Result<()> {
        self.cancel();
        crossterm::execute!(
            std::io::stderr(),
            crossterm::event::DisableMouseCapture,
            crossterm::terminal::LeaveAlternateScreen
        )?;
        crossterm::terminal::disable_raw_mode()?;
        Ok(())
    }

    pub async fn run(&mut self, app: &mut App) -> Result<()> {
        self.enter()?;
        self.start();

        // Initial data fetch
        app.fetch_all().await;

        loop {
            if let Some(action) = self.action_rx.recv().await {
                match action {
                    Action::Render => {
                        self.terminal.draw(|f| ui::draw(f, app))?;
                    }
                    Action::Tick => {
                        app.fetch_all().await;
                        if app.check_score_alerts() {
                            eprint!("\x07");
                        }
                    }
                    Action::Key(key) => {
                        if let Some(a) = app.handle_key(key).await {
                            let _ = self.action_tx.send(a);
                        }
                    }
                    _ => {}
                }
            }

            if app.should_quit {
                break;
            }
        }

        self.exit()?;
        Ok(())
    }
}

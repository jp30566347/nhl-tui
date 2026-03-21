use ratatui::crossterm::event::KeyEvent;

#[derive(Debug, Clone)]
pub enum Action {
    Tick,
    Render,
    Key(KeyEvent),
}

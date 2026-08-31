use ratatui::crossterm::event::KeyEvent;

use crate::app::Fetched;

#[derive(Debug)]
pub enum Action {
    /// Redraw the screen.
    Render,
    /// Kick off a background fetch of every panel's data.
    Refresh,
    /// A background fetch finished. Boxed: the payload is large and this
    /// variant would otherwise dominate the size of every `Action`.
    Fetched(Box<Fetched>),
    Key(KeyEvent),
}

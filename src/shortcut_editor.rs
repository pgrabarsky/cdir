use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent};
use log::{debug, error};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Clear, Paragraph},
};
use tui_textarea::{Input, TextArea};

use crate::{
    config::Config,
    store,
    store::Shortcut,
    tui::{EventCaptured, ManagerAction, View},
};

pub struct ShortcutEditor {
    store: store::Store,
    config: Arc<Config>,
    shortcut: Option<Shortcut>,
    textarea: Option<TextArea<'static>>,
}

impl ShortcutEditor {
    pub fn new(store: store::Store, config: Arc<Config>, shortcut: Shortcut) -> Self {
        Self {
            store,
            config,
            shortcut: Some(shortcut),
            textarea: None,
        }
    }
}

impl View for ShortcutEditor {
    fn init(&mut self) {
        debug!("Initializing ShortcutEditor view");
        let mut textarea = TextArea::default();
        textarea.set_block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(
                    "Description of '{}'",
                    self.shortcut.clone().unwrap().name
                ))
                .title_style(self.config.styles.title_style)
                .border_style(Style::default().fg(self.config.styles.border_color.unwrap())),
        );
        textarea.set_cursor_line_style(self.config.styles.text_style);
        if let Some(description) = self.shortcut.as_ref().unwrap().description.as_ref() {
            textarea.insert_str(description.as_str());
        }
        self.textarea = Some(textarea);
    }

    fn draw(&mut self, frame: &mut Frame, _area: Rect, _active: bool) {
        debug!("Drawing shortcut editor");

        // Only draw if we have a textarea initialized
        if self.textarea.is_none() {
            return;
        }

        let layout = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(3),
            Constraint::Fill(1),
        ]);
        let chunks = layout.split(frame.area());

        let center_layout = Layout::horizontal([
            Constraint::Length(10),
            Constraint::Min(10),
            Constraint::Length(10),
        ]);
        let chunks = center_layout.split(chunks[1]);

        frame.render_widget(Clear, chunks[1]);
        // Fill the frame with the background color if defined
        if let Some(bg_color) = &self.config.styles.background_color {
            let background = Paragraph::new("").style(Style::default().bg(*bg_color));
            frame.render_widget(background, chunks[1]);
        }

        frame.render_widget(self.textarea.as_ref().unwrap(), chunks[1]);
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> (EventCaptured, ManagerAction) {
        debug!("Handling key event: {:?}", key_event);
        let mut close = false;

        match key_event.code {
            KeyCode::Esc => {
                self.shortcut = None;
                self.textarea = None;
                close = true;
            }
            KeyCode::Enter => {
                debug!("Saving shortcut description");
                if let Some(textarea) = self.textarea.as_ref() {
                    let description = if textarea.lines().is_empty() {
                        None
                    } else {
                        Some(textarea.lines()[0].as_str())
                    };
                    debug!("Saving description: {:?}", description);
                    if let Some(shortcut) = self.shortcut.as_ref()
                        && let Err(err) = self.store.update_shortcut(
                            shortcut.id,
                            shortcut.name.as_str(),
                            shortcut.path.as_str(),
                            description,
                        )
                    {
                        error!("Error updating shortcut: {}", err);
                    }
                }
                self.shortcut = None;
                self.textarea = None;
                close = true;
            }
            _ => {
                if let Some(textarea) = self.textarea.as_mut() {
                    textarea.input(Input::from(key_event));
                }
            }
        }

        (
            EventCaptured::Yes,
            ManagerAction::new(true).with_close(close),
        )
    }
}

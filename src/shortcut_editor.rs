use crate::store;
use crate::store::Shortcut;
use crate::tableview::ModalView;
use crossterm::event::{Event, KeyCode};
use log::{debug, error};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Clear};
use ratatui::Frame;
use tui_textarea::{Input, Key, TextArea};

pub struct ShortcutEditor {
    store: store::Store,
    shortcut: Option<Shortcut>,
    textarea: Option<TextArea<'static>>,
}

impl ShortcutEditor {
    pub fn new(store: store::Store) -> Self {
        Self {
            store,
            shortcut: None,
            textarea: None,
        }
    }
}

impl ModalView<Shortcut> for ShortcutEditor {
    fn initialize(&mut self, item: &Shortcut) {
        debug!("Initializing shortcut {}", item);
        self.shortcut = Some(item.clone());
        let mut textarea = TextArea::default();
        textarea.set_block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Description of '{}'", item.name.clone())),
        );
        if let Some(description) = self.shortcut.as_ref().unwrap().description.as_ref() {
            textarea.insert_str(description.as_str());
        }
        self.textarea = Some(textarea);
    }

    fn handle_event(&mut self, event: Event) -> bool {
        debug!("Handling event: {:?}", event);
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Esc => {
                    self.shortcut = None;
                    self.textarea = None;
                    return false;
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
                        if let (Some(shortcut)) = self.shortcut.as_ref() {
                            if let Err(err) = self.store.update_shortcut(
                                shortcut.id,
                                shortcut.name.as_str(),
                                shortcut.path.as_str(),
                                description,
                            ) {
                                error!("Error updating shortcut: {}", err);
                            }
                        }
                    }
                    self.shortcut = None;
                    self.textarea = None;
                    return false;
                }
                _ => {
                    self.textarea.as_mut().unwrap().input(Input::from(key));
                }
            }
        }

        // Continue handling events
        true
    }

    fn draw(&mut self, frame: &mut Frame) {
        debug!("Drawing shortcut");
        let layout = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(3),
            Constraint::Fill(1),
        ]);
        let chunks = layout.split(frame.area());
        frame.render_widget(Clear, chunks[1]);
        frame.render_widget(self.textarea.as_ref().unwrap(), chunks[1]);
    }
}

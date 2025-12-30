use crate::config::Config;
use crate::store;
use crate::store::Shortcut;
use crate::tableview::ModalView;
use crossterm::event::{Event, KeyCode};
use log::{debug, error};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use tui_textarea::{Input, TextArea};

pub struct ShortcutEditor {
    store: store::Store,
    config: Config,
    shortcut: Option<Shortcut>,
    textarea: Option<TextArea<'static>>,
}

impl ShortcutEditor {
    pub fn new(store: store::Store, config: Config) -> Self {
        Self {
            store,
            config,
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
                .title(format!("Description of '{}'", item.name.clone()))
                .title_style(self.config.styles.title_style)
                .border_style(Style::default().fg(self.config.styles.border_color.unwrap())),
        );
        textarea.set_cursor_line_style(self.config.styles.text_style);
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
}

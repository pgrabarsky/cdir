use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent};
use log::{debug, error};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph},
};
use tui_textarea::{Input, TextArea};

use crate::{
    config::Config,
    store,
    store::Shortcut,
    tui::{EventCaptured, ManagerAction, View, ViewBuilder},
};

#[derive(Copy, Clone, PartialEq)]
enum EditorField {
    Name,
    Description,
    YesButton,
    CancelButton,
}

pub struct ShortcutEditor {
    store: store::Store,
    config: Arc<Config>,
    shortcut: Option<Shortcut>,
    name_textarea: Option<TextArea<'static>>,
    description_textarea: Option<TextArea<'static>>,
    selected_field: EditorField,
}

impl ShortcutEditor {
    pub fn builder(store: store::Store, config: Arc<Config>, shortcut: Shortcut) -> ViewBuilder {
        ViewBuilder::from(Box::new(Self {
            store,
            config,
            shortcut: Some(shortcut),
            name_textarea: None,
            description_textarea: None,
            selected_field: EditorField::Name,
        }))
    }

    fn save_shortcut(&mut self) {
        // Save the shortcut
        debug!("Saving shortcut");
        if let Some(name_textarea) = self.name_textarea.as_ref()
            && let Some(description_textarea) = self.description_textarea.as_ref()
        {
            let name = if name_textarea.lines().is_empty() {
                ""
            } else {
                name_textarea.lines()[0].as_str()
            };

            let description = if description_textarea.lines().is_empty() {
                None
            } else {
                Some(description_textarea.lines()[0].as_str())
            };

            debug!("Saving name: {:?}, description: {:?}", name, description);
            if let Some(shortcut) = self.shortcut.as_ref()
                && let Err(err) = self.store.update_shortcut(
                    shortcut.id,
                    name,
                    shortcut.path.as_str(),
                    description,
                )
            {
                error!("Error updating shortcut: {}", err);
            }
        }
    }
}

impl View for ShortcutEditor {
    fn init(&mut self) {
        debug!("Initializing ShortcutEditor view");

        // Initialize name textarea
        let mut name_textarea = TextArea::default();
        name_textarea.set_block(
            Block::default()
                .borders(Borders::ALL)
                .title("Name")
                .title_style(self.config.styles.title_style)
                .border_style(Style::default().fg(self.config.styles.border_color.unwrap())),
        );
        name_textarea.set_cursor_line_style(self.config.styles.text_style);
        if let Some(shortcut) = self.shortcut.as_ref() {
            name_textarea.insert_str(shortcut.name.as_str());
        }
        self.name_textarea = Some(name_textarea);

        // Initialize description textarea
        let mut description_textarea = TextArea::default();
        description_textarea.set_block(
            Block::default()
                .borders(Borders::ALL)
                .title("Description")
                .title_style(self.config.styles.title_style)
                .border_style(Style::default().fg(self.config.styles.border_color.unwrap())),
        );
        description_textarea.set_cursor_line_style(self.config.styles.text_style);
        if let Some(description) = self.shortcut.as_ref().unwrap().description.as_ref() {
            description_textarea.insert_str(description.as_str());
        }
        self.description_textarea = Some(description_textarea);
    }

    fn draw(&mut self, frame: &mut Frame, modal_area: Rect, _active: bool) {
        debug!("Drawing shortcut editor");

        // Only draw if we have textareas initialized
        if self.name_textarea.is_none() || self.description_textarea.is_none() {
            return;
        }

        // Create a modal that's centered on the screen
        let layout = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(12),
            Constraint::Fill(1),
        ]);
        let chunks = layout.split(modal_area);
        let center_layout = Layout::horizontal([
            Constraint::Fill(5),
            Constraint::Length(80),
            Constraint::Fill(5),
        ]);
        let chunks = center_layout.split(chunks[1]);
        let modal_area = chunks[1];

        frame.render_widget(Clear, modal_area);
        // Fill the frame with the background color if defined
        if let Some(bg_color) = &self.config.styles.background_color {
            let background = Paragraph::new("").style(Style::default().bg(*bg_color));
            frame.render_widget(background, modal_area);
        }

        // Draw the outer border
        let block = Block::default()
            .title("Edit Shortcut")
            .title_style(self.config.styles.title_style)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.config.styles.border_color.unwrap()))
            .style(self.config.styles.text_style);
        frame.render_widget(block, modal_area);

        // Split modal into name, description, and buttons
        let inner = Rect {
            x: modal_area.x + 1,
            y: modal_area.y + 1,
            width: modal_area.width - 2,
            height: modal_area.height - 2,
        };
        let vchunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // name textarea
                Constraint::Length(3), // description textarea
                Constraint::Fill(1),   // spacing
                Constraint::Length(1), // buttons
            ])
            .split(inner);

        // Update border styles and cursor visibility based on selected field
        if let Some(name_textarea) = self.name_textarea.as_mut() {
            name_textarea.set_block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Name")
                    .title_style(self.config.styles.text_style)
                    .border_style(Style::default().fg(self.config.styles.border_color.unwrap())),
            );
            // Show cursor only if this field is selected
            if self.selected_field == EditorField::Name {
                name_textarea.set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));
            } else {
                name_textarea.set_cursor_style(Style::default());
            }
        }
        if let Some(name_textarea) = self.name_textarea.as_ref() {
            frame.render_widget(name_textarea, vchunks[0]);
        }

        if let Some(description_textarea) = self.description_textarea.as_mut() {
            description_textarea.set_block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Description")
                    .title_style(self.config.styles.text_style)
                    .border_style(Style::default().fg(self.config.styles.border_color.unwrap())),
            );
            // Show cursor only if this field is selected
            if self.selected_field == EditorField::Description {
                description_textarea
                    .set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));
            } else {
                description_textarea.set_cursor_style(Style::default());
            }
        }
        if let Some(description_textarea) = self.description_textarea.as_ref() {
            frame.render_widget(description_textarea, vchunks[1]);
        }

        // Buttons at the bottom
        let button_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(10),
                Constraint::Fill(1),
                Constraint::Length(10),
            ])
            .split(vchunks[3]);

        let yes_style = if self.selected_field == EditorField::YesButton {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };
        let cancel_style = if self.selected_field == EditorField::CancelButton {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Red)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Red)
        };
        let yes = Paragraph::new(" Yes ")
            .style(yes_style)
            .alignment(ratatui::layout::Alignment::Center);
        let cancel = Paragraph::new(" Cancel ")
            .style(cancel_style)
            .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(yes, button_layout[0]);
        frame.render_widget(cancel, button_layout[2]);
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> (EventCaptured, ManagerAction) {
        debug!("Handling key event: {:?}", key_event);
        let mut close = false;
        let mut redraw = false;

        match key_event.code {
            KeyCode::Esc => {
                // Cancel - close without saving
                self.shortcut = None;
                self.name_textarea = None;
                self.description_textarea = None;
                close = true;
            }
            KeyCode::Tab => {
                // Navigate between fields
                self.selected_field = match self.selected_field {
                    EditorField::Name => EditorField::Description,
                    EditorField::Description => EditorField::YesButton,
                    EditorField::YesButton => EditorField::CancelButton,
                    EditorField::CancelButton => EditorField::Name,
                };
                redraw = true;
            }
            KeyCode::BackTab => {
                // Navigate backwards between fields (Shift+Tab)
                self.selected_field = match self.selected_field {
                    EditorField::Name => EditorField::CancelButton,
                    EditorField::Description => EditorField::Name,
                    EditorField::YesButton => EditorField::Description,
                    EditorField::CancelButton => EditorField::YesButton,
                };
                redraw = true;
            }
            KeyCode::Left | KeyCode::Right
                if matches!(
                    self.selected_field,
                    EditorField::YesButton | EditorField::CancelButton
                ) =>
            {
                // Toggle between Yes and Cancel buttons
                self.selected_field = match self.selected_field {
                    EditorField::YesButton => EditorField::CancelButton,
                    EditorField::CancelButton => EditorField::YesButton,
                    _ => self.selected_field,
                };
                redraw = true;
            }
            KeyCode::Enter => {
                // Handle button press
                if self.selected_field != EditorField::CancelButton {
                    self.save_shortcut();
                }
                // Close regardless of Yes or Cancel
                self.shortcut = None;
                self.name_textarea = None;
                self.description_textarea = None;
                close = true;
            }
            _ => {
                // Handle input for the selected textarea
                match self.selected_field {
                    EditorField::Name => {
                        if let Some(textarea) = self.name_textarea.as_mut() {
                            textarea.input(Input::from(key_event));
                        }
                    }
                    EditorField::Description => {
                        if let Some(textarea) = self.description_textarea.as_mut() {
                            textarea.input(Input::from(key_event));
                        }
                    }
                    _ => {}
                }
                redraw = true;
            }
        }

        (
            EventCaptured::Yes,
            ManagerAction::new(redraw).with_close(close),
        )
    }
}

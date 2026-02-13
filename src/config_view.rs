use std::{
    rc::Rc,
    sync::{Arc, Mutex},
};

use crossterm::event::{KeyCode, KeyEvent};
use log::{error, info};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph},
};
use tokio::sync::broadcast::Sender;
use tui_textarea::{Input, TextArea};

use crate::{
    config::Config,
    tui::{
        EventCaptured, GenericEvent, ManagerAction, View, ViewBuilder, ViewManager,
        event::ApplicationEvent,
    },
};

#[derive(Copy, Clone, PartialEq)]
enum ConfigField {
    SmartSuggestionActiveCheckbox,
    PathSearchCheckbox,
    SmartSuggestionCountField,
    YesButton,
    CancelButton,
}

pub struct ConfigView {
    tx: Sender<GenericEvent>,
    config: Arc<Mutex<Config>>,
    smart_suggestions_field: ConfigField,
    smart_suggestions_active: bool,
    path_search_include_shortcuts: bool,
    count_textarea: Option<TextArea<'static>>,
}

impl ConfigView {
    pub fn builder(view_manager: Rc<ViewManager>, config: Arc<Mutex<Config>>) -> ViewBuilder {
        ViewBuilder::from(Box::new(Self {
            tx: view_manager.tx(),
            config,
            smart_suggestions_field: ConfigField::SmartSuggestionActiveCheckbox,
            smart_suggestions_active: false,
            path_search_include_shortcuts: false,
            count_textarea: None,
        }))
    }

    fn save_config(&mut self) {
        if let Ok(mut config) = self.config.lock() {
            info!(
                "Saving config: smart_suggestions_active={}",
                self.smart_suggestions_active
            );
            config.smart_suggestions_active = self.smart_suggestions_active;
            config.path_search_include_shortcuts = self.path_search_include_shortcuts;

            // Save smart_suggestions_count from TextArea
            if let Some(textarea) = &self.count_textarea
                && !textarea.lines().is_empty()
            {
                let count_str = textarea.lines()[0].as_str();
                if let Ok(count) = count_str.parse::<usize>() {
                    info!("Saving smart_suggestions_count={}", count);
                    config.smart_suggestions_count = count;
                } else {
                    error!("Invalid count value '{}', keeping current value", count_str);
                }
            }
            if let Err(e) = config.save() {
                error!("Failed to save config: {}", e);
            }
            self.publish();
        } else {
            error!("Failed to lock config to save state");
        }
    }

    fn cancel_config(&mut self) {
        // nop
    }

    fn publish(&self) {
        let event = GenericEvent::ApplicationEvent(ApplicationEvent {
            id: String::from("data.reload"),
            payload: None,
        });
        let result = self.tx.send(event);
        if let Err(e) = result {
            error!("Failed to send 'data.reload' event: {}", e);
        }
    }
}

impl View for ConfigView {
    fn init(&mut self) {
        if let Ok(config_lock) = self.config.lock() {
            self.smart_suggestions_active = config_lock.smart_suggestions_active;
            self.path_search_include_shortcuts = config_lock.path_search_include_shortcuts;

            // Initialize count textarea
            let mut count_textarea = TextArea::default();
            count_textarea.set_block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Smart Suggestions Count")
                    .title_style(config_lock.styles.title_style)
                    .border_style(Style::default().fg(config_lock.styles.border_color.unwrap())),
            );
            count_textarea.set_cursor_line_style(config_lock.styles.text_style);
            count_textarea.insert_str(config_lock.smart_suggestions_count.to_string());
            self.count_textarea = Some(count_textarea);
        } else {
            error!("Failed to lock config to get initial state, using defaults");
            self.smart_suggestions_active = false;

            // Initialize with default textarea
            let mut count_textarea = TextArea::default();
            count_textarea.set_block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Smart Suggestions Count"),
            );
            count_textarea.insert_str("3"); // Default value
            self.count_textarea = Some(count_textarea);
        };
    }
    fn handle_key_event(&mut self, key_event: KeyEvent) -> (EventCaptured, ManagerAction) {
        let mut close = false;

        match key_event.code {
            KeyCode::Esc => {
                // Cancel - restore original state and close
                self.cancel_config();
                close = true;
            }
            KeyCode::Tab | KeyCode::Down => {
                // Navigate between fields
                self.smart_suggestions_field = match self.smart_suggestions_field {
                    ConfigField::SmartSuggestionActiveCheckbox => {
                        ConfigField::SmartSuggestionCountField
                    }
                    ConfigField::SmartSuggestionCountField => ConfigField::PathSearchCheckbox,
                    ConfigField::PathSearchCheckbox => ConfigField::YesButton,
                    ConfigField::YesButton => ConfigField::CancelButton,
                    ConfigField::CancelButton => ConfigField::SmartSuggestionActiveCheckbox,
                };
            }
            KeyCode::BackTab | KeyCode::Up => {
                // Navigate backwards between fields (Shift+Tab)
                self.smart_suggestions_field = match self.smart_suggestions_field {
                    ConfigField::SmartSuggestionActiveCheckbox => ConfigField::CancelButton,
                    ConfigField::SmartSuggestionCountField => {
                        ConfigField::SmartSuggestionActiveCheckbox
                    }
                    ConfigField::PathSearchCheckbox => ConfigField::SmartSuggestionCountField,
                    ConfigField::YesButton => ConfigField::PathSearchCheckbox,
                    ConfigField::CancelButton => ConfigField::YesButton,
                };
            }
            KeyCode::Left | KeyCode::Right
                if matches!(
                    self.smart_suggestions_field,
                    ConfigField::YesButton | ConfigField::CancelButton
                ) =>
            {
                // Toggle between Yes and Cancel buttons
                self.smart_suggestions_field = match self.smart_suggestions_field {
                    ConfigField::YesButton => ConfigField::CancelButton,
                    ConfigField::CancelButton => ConfigField::YesButton,
                    _ => self.smart_suggestions_field,
                };
            }
            KeyCode::Char(' ')
                if self.smart_suggestions_field == ConfigField::SmartSuggestionActiveCheckbox =>
            {
                self.smart_suggestions_active = !self.smart_suggestions_active;
            }
            KeyCode::Char(' ')
                if self.smart_suggestions_field == ConfigField::PathSearchCheckbox =>
            {
                self.path_search_include_shortcuts = !self.path_search_include_shortcuts;
            }
            KeyCode::Enter => {
                match self.smart_suggestions_field {
                    ConfigField::SmartSuggestionActiveCheckbox => {
                        // Toggle smart suggestions
                        self.smart_suggestions_active = !self.smart_suggestions_active;
                    }
                    ConfigField::PathSearchCheckbox => {
                        // Toggle path search include shortcuts
                        self.path_search_include_shortcuts = !self.path_search_include_shortcuts;
                    }
                    ConfigField::SmartSuggestionCountField => {
                        // Move to next field (Yes button) when Enter is pressed on TextArea
                        self.smart_suggestions_field = ConfigField::YesButton;
                    }
                    ConfigField::YesButton => {
                        // Save and close
                        self.save_config();
                        close = true;
                    }
                    ConfigField::CancelButton => {
                        // Cancel and close
                        self.cancel_config();
                        close = true;
                    }
                }
            }
            _ => {
                // Handle input for the count textarea - only allow numeric input and navigation
                if self.smart_suggestions_field == ConfigField::SmartSuggestionCountField
                    && let Some(textarea) = self.count_textarea.as_mut()
                {
                    match key_event.code {
                        // Allow numeric keys (but limit to 3 characters)
                        KeyCode::Char(c) if c.is_ascii_digit() => {
                            let current_text = if textarea.lines().is_empty() {
                                String::new()
                            } else {
                                textarea.lines()[0].clone()
                            };
                            if current_text.len() < 2 {
                                textarea.input(Input::from(key_event));
                            }
                        }
                        // Allow text editing keys
                        KeyCode::Backspace | KeyCode::Delete => {
                            textarea.input(Input::from(key_event));
                        }
                        // Allow cursor navigation within the field
                        KeyCode::Left | KeyCode::Right | KeyCode::Home | KeyCode::End => {
                            textarea.input(Input::from(key_event));
                        }
                        // Ignore all other keys
                        _ => {}
                    }
                }
            }
        }

        (
            EventCaptured::Yes,
            ManagerAction::new(true).with_close(close),
        )
    }

    fn handle_mouse_event(
        &mut self,
        _area: Rect,
        _mouse_event: crossterm::event::MouseEvent,
    ) -> ManagerAction {
        ManagerAction::new(false)
    }

    fn draw(&mut self, frame: &mut ratatui::Frame, modal_area: Rect, _active: bool) {
        let layout = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(9),
            Constraint::Fill(1),
        ]);
        let chunks: [Rect; 3] = layout.areas(modal_area);
        let center_layout = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Length(60),
            Constraint::Fill(1),
        ]);
        let chunks: [Rect; 3] = center_layout.areas(chunks[1]);
        let modal_area = chunks[1];

        frame.render_widget(Clear, modal_area);

        let config_lock = self.config.lock().unwrap();

        // Fill the frame with the background color if defined
        if let Some(bg_color) = &config_lock.styles.background_color {
            let background = Paragraph::new("").style(Style::default().bg(*bg_color));
            frame.render_widget(background, modal_area);
        }

        let block = Block::default()
            .title("Configuration")
            .title_style(config_lock.styles.title_style)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(config_lock.styles.border_color.unwrap()))
            .style(config_lock.styles.text_style);

        frame.render_widget(block, modal_area);

        // Create content area inside the block
        let inner_area = ratatui::layout::Rect {
            x: modal_area.x + 1,
            y: modal_area.y + 1,
            width: modal_area.width.saturating_sub(2),
            height: modal_area.height.saturating_sub(2),
        };

        // Create checkbox content
        let checkbox_symbol = if self.smart_suggestions_active {
            "[X]"
        } else {
            "[ ]"
        };
        let checkbox_text = format!("{}  Smart suggestions", checkbox_symbol);

        let content_layout = Layout::vertical([
            Constraint::Length(1), // Empty line at top
            Constraint::Length(1), // Checkbox line
            Constraint::Length(1), // Path search checkbox line
            Constraint::Length(1), // TextArea (3 lines with border)
            Constraint::Length(1), // Empty line before buttons
            Constraint::Length(1), // Buttons
        ]);
        let [
            _top_spacer,
            suggestions_active_area,
            suggestions_count_area,
            path_search_area,
            _spacer3,
            buttons_area,
        ]: [Rect; 6] = content_layout.areas(inner_area);

        // Render checkbox with highlighting if selected
        let checkbox_style =
            if self.smart_suggestions_field == ConfigField::SmartSuggestionActiveCheckbox {
                config_lock
                    .styles
                    .text_style
                    .add_modifier(Modifier::REVERSED)
            } else {
                config_lock.styles.text_style
            };
        let checkbox = Paragraph::new(checkbox_text).style(checkbox_style);
        frame.render_widget(checkbox, suggestions_active_area);

        // Render count field as formatted text [XX] message
        if let Some(count_textarea) = &self.count_textarea {
            let count_value = if count_textarea.lines().is_empty() {
                "3".to_string()
            } else {
                count_textarea.lines()[0].clone()
            };

            // Pad with spaces to always show 2 characters
            let padded_count = format!("{:>2}", count_value);
            let count_display = format!("[{}] Smart suggestions count", padded_count);

            let count_style =
                if self.smart_suggestions_field == ConfigField::SmartSuggestionCountField {
                    config_lock
                        .styles
                        .text_style
                        .add_modifier(Modifier::REVERSED)
                } else {
                    config_lock.styles.text_style
                };

            let count_paragraph = Paragraph::new(count_display).style(count_style);
            frame.render_widget(count_paragraph, suggestions_count_area);
        }

        // Render path search checkbox
        let path_checkbox_symbol = if self.path_search_include_shortcuts {
            "[X]"
        } else {
            "[ ]"
        };
        let path_checkbox_text =
            format!("{}  Include shortcuts in path search", path_checkbox_symbol);
        let path_checkbox_style = if self.smart_suggestions_field == ConfigField::PathSearchCheckbox
        {
            config_lock
                .styles
                .text_style
                .add_modifier(Modifier::REVERSED)
        } else {
            config_lock.styles.text_style
        };
        let path_checkbox = Paragraph::new(path_checkbox_text).style(path_checkbox_style);
        frame.render_widget(path_checkbox, path_search_area);

        // Render buttons at the bottom
        let button_layout: [Rect; 3] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(10),
                Constraint::Fill(1),
                Constraint::Length(10),
            ])
            .areas(buttons_area);

        let yes_style = if self.smart_suggestions_field == ConfigField::YesButton {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };
        let cancel_style = if self.smart_suggestions_field == ConfigField::CancelButton {
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
}

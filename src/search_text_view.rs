use std::{
    rc::Rc,
    sync::{Arc, Mutex},
};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent};
use log::{debug, error, warn};
use ratatui::{
    layout::{Constraint, Layout, Position, Rect},
    style::Style,
    widgets::Paragraph,
};
use tokio::sync::broadcast::Sender;

use crate::{
    config::Config,
    tui::{
        EventCaptured, GenericEvent, ManagerAction, View, ViewBuilder, ViewManager,
        event::ApplicationEvent,
    },
};

const SEARCH_PROMPT: &str = "> ";

// "search.description"
pub struct SearchTextState {
    tx: Sender<GenericEvent>,
    search_string: String,
    search_string_cursor_index: usize,
    fuzzy_match: bool,
}

pub struct SearchDescriptionPayload {
    pub search_string: String,
    pub fuzzy_match: bool,
}

impl SearchTextState {
    pub fn new(view_manager: Rc<ViewManager>) -> SearchTextState {
        SearchTextState {
            tx: view_manager.tx(),
            search_string: String::new(),
            search_string_cursor_index: 0,
            fuzzy_match: false,
        }
    }

    fn publish(&self) {
        let event = GenericEvent::ApplicationEvent(ApplicationEvent {
            id: String::from("search.description"),
            payload: Some(Arc::new(SearchDescriptionPayload {
                search_string: self.search_string.clone(),
                fuzzy_match: self.fuzzy_match,
            })),
        });
        let result = self.tx.send(event);
        if let Err(e) = result {
            error!("Failed to send 'search.description' event: {}", e);
        }
    }
}

pub struct SearchTextView {
    config: Arc<Mutex<Config>>,
    state: Arc<Mutex<SearchTextState>>,
}

impl SearchTextView {
    pub fn builder(config: Arc<Mutex<Config>>, state: Arc<Mutex<SearchTextState>>) -> ViewBuilder {
        ViewBuilder::from(Box::new(SearchTextView { config, state }))
    }

    pub fn toggle_fuzzy_match(&mut self) {
        let mut state_lock = self.state.lock().unwrap();
        state_lock.fuzzy_match = !state_lock.fuzzy_match;
        state_lock.publish();
    }
}

impl View for SearchTextView {
    fn draw(&mut self, frame: &mut ratatui::Frame, area: Rect, active: bool) {
        debug!("draw area='{}' active='{}", area, active);
        let config_lock = self.config.lock().unwrap();
        // Fill the frame with the background color if defined
        if let Some(bg_color) = &config_lock.styles.free_text_area_bg_color {
            // let area = frame.area();
            let background = Paragraph::new("").style(Style::default().bg(*bg_color));
            frame.render_widget(background, area);
        }

        let input = area;

        let state_lock = self.state.lock().unwrap();
        let search_string = state_lock.search_string.clone();

        let search_text_area: Rect;
        {
            // bottom line
            let horizontal =
                Layout::horizontal([Constraint::Length(4), Constraint::Percentage(100)]).spacing(0);
            let left: Rect;
            [left, search_text_area] = horizontal.areas(input);

            // The left exact/fuzzy indicator

            let mut pa = if state_lock.fuzzy_match {
                Paragraph::new("[f]")
            } else {
                Paragraph::new("[e]")
            };
            pa = pa.style(
                config_lock
                    .styles
                    .date_style
                    .bg(config_lock.styles.free_text_area_bg_color.unwrap()),
            );
            frame.render_widget(pa, left);

            // Draw the free text area

            let pa = Paragraph::new(format!("{}{}", SEARCH_PROMPT, search_string.as_str())).style(
                config_lock
                    .styles
                    .path_style
                    .bg(config_lock.styles.free_text_area_bg_color.unwrap()),
            );
            frame.render_widget(pa, search_text_area);
        }

        if active {
            // Don't activate the cursor if not active...
            let search_string_cursor_index = state_lock.search_string_cursor_index;
            frame.set_cursor_position(Position::new(
                search_text_area.x + search_string_cursor_index as u16 + SEARCH_PROMPT.len() as u16,
                search_text_area.y,
            ));
        }
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> (EventCaptured, ManagerAction) {
        let _ = key_event;
        debug!("handle_key_event");

        match key_event.code {
            KeyCode::Backspace => {
                let mut state_lock = self.state.lock().unwrap();
                if state_lock.search_string_cursor_index != 0 {
                    let search_string_cursor_index = state_lock.search_string_cursor_index;
                    state_lock.search_string_cursor_index -= 1;
                    state_lock
                        .search_string
                        .remove(search_string_cursor_index - 1);
                    state_lock.publish();
                }
            }
            KeyCode::Delete => {
                let mut state_lock = self.state.lock().unwrap();
                if state_lock.search_string_cursor_index < state_lock.search_string.len() {
                    let search_string_cursor_index = state_lock.search_string_cursor_index;
                    state_lock.search_string.remove(search_string_cursor_index);
                    state_lock.publish();
                }
            }
            KeyCode::Left => {
                let mut state_lock = self.state.lock().unwrap();
                if state_lock.search_string_cursor_index != 0 {
                    state_lock.search_string_cursor_index -= 1;
                }
            }
            KeyCode::Right => {
                let mut state_lock = self.state.lock().unwrap();
                if state_lock.search_string_cursor_index < state_lock.search_string.len() {
                    state_lock.search_string_cursor_index += 1;
                }
            }
            KeyCode::Char(c) => {
                if key_event.modifiers != KeyModifiers::CONTROL {
                    let mut state_lock = self.state.lock().unwrap();
                    let search_string_cursor_index = state_lock.search_string_cursor_index;
                    state_lock
                        .search_string
                        .insert(search_string_cursor_index, c);
                    state_lock.search_string_cursor_index += 1;
                    state_lock.publish();
                } else if c == 'f' {
                    self.toggle_fuzzy_match();
                }
            }
            _ => {
                warn!("Unknown action key={}", key_event.code);
            }
        }

        (EventCaptured::No, ManagerAction::new(false))
    }

    fn handle_mouse_event(&mut self, area: Rect, mouse_event: MouseEvent) -> ManagerAction {
        let mut ma = ManagerAction::new(false);

        if !matches!(mouse_event.kind, crossterm::event::MouseEventKind::Down(_)) {
            return ma;
        }

        let position = Position::new(mouse_event.column, mouse_event.row);

        // Check if the mouse event is within the view's area
        if !area.contains(position) {
            return ma;
        }

        // Toggle fuzzy/exact match on click
        self.toggle_fuzzy_match();

        ma.redraw = true;
        ma
    }
}

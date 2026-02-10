use std::{
    rc::Rc,
    sync::{Arc, Mutex},
};

use log::debug;
use ratatui::{
    layout::{Alignment, Position, Rect},
    prelude::Style,
    style::{Color, Stylize},
    widgets::Paragraph,
};

use crate::{
    config::Config,
    help::Help,
    model::DataStatePayload,
    tui::{ManagerAction, View, ViewBuilder, ViewManager, event::ApplicationEvent},
};

pub struct ListIndicatorState {
    objects_type: String,
    is_empty: bool,
}

impl ListIndicatorState {
    pub fn new(objects_type: String) -> ListIndicatorState {
        Self {
            objects_type,
            is_empty: false,
        }
    }
}

pub struct ListIndicatorView {
    vm: Rc<ViewManager>,
    state: ListIndicatorState,
    config: Arc<Mutex<Config>>,
}

impl ListIndicatorView {
    pub fn builder(
        vm: Rc<ViewManager>,
        config: Arc<Mutex<Config>>,
        objects_type: String,
    ) -> ViewBuilder {
        ViewBuilder::from(Box::new(ListIndicatorView {
            vm,
            state: ListIndicatorState::new(objects_type),
            config,
        }))
        .with_publish_events(true)
    }
}

impl View for ListIndicatorView {
    fn draw(&mut self, frame: &mut ratatui::Frame, area: Rect, _active: bool) {
        let config_lock = self.config.lock().unwrap();
        // Fill the frame with the background color if defined
        if let Some(bg_color) = &config_lock.styles.background_color {
            // let area = frame.area();
            let background = Paragraph::new("").style(Style::default().bg(*bg_color));
            frame.render_widget(background, area);
        }

        let pa = if self.state.is_empty {
            Paragraph::new("no entry")
                .style(
                    Style::default()
                        .fg(Color::Black)
                        .bg(config_lock.styles.free_text_area_bg_color.unwrap()),
                )
                .bg(Color::Red)
                .alignment(Alignment::Center)
        } else {
            Paragraph::new("ctrl+h: help")
                .style(
                    Style::default()
                        .bg(config_lock.styles.header_bg_color.unwrap())
                        .fg(config_lock.styles.header_fg_color.unwrap()),
                )
                .alignment(Alignment::Center)
        };
        frame.render_widget(pa, area);
    }
    fn handle_mouse_event(
        &mut self,
        area: Rect,
        mouse_event: crossterm::event::MouseEvent,
    ) -> crate::tui::ManagerAction {
        let mouse_position = Position::new(mouse_event.column, mouse_event.row);
        if mouse_event.kind
            == crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left)
            && area.contains(mouse_position)
        {
            self.vm
                .show_modal_generic(Help::builder(self.config.clone()), None);
            return ManagerAction::new(true);
        }
        ManagerAction::new(false)
    }
    fn handle_application_event(&mut self, ae: &ApplicationEvent) {
        debug!("handle_application_event");
        if ae.id == "data.payload"
            && let Some(payload) = &ae.payload
            && let Some(payload) = payload.downcast_ref::<DataStatePayload>()
            && payload.objects_type == self.state.objects_type
        {
            debug!("data.payload is_empty={}", payload.is_empty);
            self.state.is_empty = payload.is_empty;

            // let _ = self
            //     .tx
            //     .send(GenericEvent::ViewManagerEvent(ViewManagerEvent::Redraw));
        }
    }
}

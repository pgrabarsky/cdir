use std::sync::Arc;

use log::debug;
use ratatui::{
    layout::{Alignment, Rect},
    prelude::Style,
    style::{Color, Stylize},
    widgets::Paragraph,
};

use crate::{
    config::Config,
    model::DataStatePayload,
    tui::{View, ViewBuilder, event::ApplicationEvent},
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
    state: ListIndicatorState,
    config: Arc<Config>,
}

impl ListIndicatorView {
    pub fn builder(config: Arc<Config>, objects_type: String) -> ViewBuilder {
        ViewBuilder::from(Box::new(ListIndicatorView {
            state: ListIndicatorState::new(objects_type),
            config,
        }))
        .with_publish_events(true)
    }
}

impl View for ListIndicatorView {
    fn draw(&mut self, frame: &mut ratatui::Frame, area: Rect, _active: bool) {
        // Fill the frame with the background color if defined
        if let Some(bg_color) = &self.config.styles.background_color {
            // let area = frame.area();
            let background = Paragraph::new("").style(Style::default().bg(*bg_color));
            frame.render_widget(background, area);
        }

        let pa = if self.state.is_empty {
            Paragraph::new("no entry")
                .style(
                    Style::default().fg(Color::Black).bg(self
                        .config
                        .styles
                        .free_text_area_bg_color
                        .unwrap()),
                )
                .bg(Color::Red)
                .alignment(Alignment::Center)
        } else {
            Paragraph::new("")
                .style(
                    Style::default().fg(Color::Black).bg(self
                        .config
                        .styles
                        .free_text_area_bg_color
                        .unwrap()),
                )
                .alignment(Alignment::Center)
        };
        frame.render_widget(pa, area);
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

use std::{
    rc::Rc,
    sync::{Arc, Mutex},
};

use ratatui::{
    layout::{Alignment, Position, Rect},
    prelude::Style,
    widgets::Paragraph,
};

use crate::{
    config::Config,
    config_view::ConfigView,
    tui::{ManagerAction, View, ViewBuilder, ViewManager},
};

pub struct ConfigButton {
    vm: Rc<ViewManager>,
    config: Arc<Mutex<Config>>,
}

impl ConfigButton {
    pub fn builder(vm: Rc<ViewManager>, config: Arc<Mutex<Config>>) -> ViewBuilder {
        ViewBuilder::from(Box::new(ConfigButton { config, vm }))
    }
}

impl View for ConfigButton {
    fn draw(&mut self, frame: &mut ratatui::Frame, area: Rect, _active: bool) {
        let config_lock = self.config.lock().unwrap();
        // Fill the frame with the background color if defined
        if let Some(bg_color) = &config_lock.styles.background_color {
            // let area = frame.area();
            let background = Paragraph::new("").style(Style::default().bg(*bg_color));
            frame.render_widget(background, area);
        }

        let pa = Paragraph::new("Config")
            .style(
                Style::default()
                    .bg(config_lock.styles.header_bg_color.unwrap())
                    .fg(config_lock.styles.header_fg_color.unwrap()),
            )
            .alignment(Alignment::Center);
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
            self.vm.show_modal_generic(
                ConfigView::builder(self.vm.clone(), self.config.clone()),
                None,
            );
            return ManagerAction::new(true);
        }
        ManagerAction::new(false)
    }
}

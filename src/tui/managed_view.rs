use std::{
    cell::RefCell,
    fmt::Formatter,
    rc::Rc,
    sync::atomic::{AtomicUsize, Ordering},
};

use log::trace;
use ratatui::{layout::Rect, widgets::Clear};

use crate::tui::view::View;

static VIEW_UNIQUE_ID_COUNTER: std::sync::atomic::AtomicUsize = AtomicUsize::new(0);

pub(super) struct ManagedView {
    pub id: usize,
    pub unique_id: String,
    pub view: Box<dyn View>,
    pub area: Rect,
    pub children: Vec<Rc<RefCell<ManagedView>>>,
    pub publish_events: bool,
}

impl PartialEq for ManagedView {
    fn eq(&self, other: &Self) -> bool { self.id == other.id }
}

impl std::fmt::Debug for ManagedView {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ManagedView")
            .field("unique_id", &self.unique_id)
            .finish()
    }
}

impl ManagedView {
    pub fn new(view: Box<dyn View>) -> Self {
        ManagedView {
            id: 0,
            unique_id: format!("{}", VIEW_UNIQUE_ID_COUNTER.fetch_add(1, Ordering::SeqCst)),
            view,
            area: Rect::default(),
            children: vec![],
            publish_events: false,
        }
    }
    pub fn add_view(&mut self, id: u16, mut v: ManagedView) -> Rc<RefCell<ManagedView>> {
        v.id = id as usize;
        self.children.push(Rc::new(RefCell::new(v)));
        self.children
            .last()
            .expect("children vec should not be empty after push")
            .clone()
    }

    pub(super) fn draw(&mut self, frame: &mut ratatui::Frame, active_view_id: Option<String>) {
        trace!("ManagedView {} draw area {:?}", self.id, self.area);

        let is_active = active_view_id
            .as_ref()
            .is_some_and(|active_view_id| active_view_id.eq(&self.unique_id));
        self.view.draw(frame, self.area, is_active);
        for child in &self.children {
            frame.render_widget(Clear, child.borrow().area);
            child.borrow_mut().draw(frame, active_view_id.clone());
        }
        trace!("exit ManagedView {} draw", self.id);
    }
}

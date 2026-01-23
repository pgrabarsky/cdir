use crate::tui::{managed_view::ManagedView, view::View};

pub struct ViewBuilder {
    view: Box<dyn View>,
    children: Vec<(u16, ViewBuilder)>,
    publish_events: bool,
}

#[allow(unused)]
impl ViewBuilder {
    pub(crate) fn from(view: Box<dyn View>) -> ViewBuilder {
        ViewBuilder {
            view,
            children: Vec::new(),
            publish_events: false,
        }
    }
    pub(crate) fn child(mut self, id: u16, child_view: ViewBuilder) -> ViewBuilder {
        self.children.push((id, child_view));
        self
    }

    pub(crate) fn with_publish_events(mut self, publis_events: bool) -> ViewBuilder {
        self.publish_events = publis_events;
        self
    }

    pub(super) fn build(self) -> ManagedView {
        let mut mv = ManagedView::new(self.view);
        for child in self.children {
            mv.add_view(child.0, child.1.build());
        }
        mv.publish_events = self.publish_events;
        mv
    }
}

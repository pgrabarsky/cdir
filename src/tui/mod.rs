pub(crate) use crate::tui::{
    event::GenericEvent,
    view::{EventCaptured, ManagerAction, View},
    view_builder::ViewBuilder,
    view_manager::ViewManager,
};

pub(crate) mod event;
pub(crate) mod managed_view;
pub(crate) mod view;
pub(crate) mod view_builder;
pub(crate) mod view_manager;

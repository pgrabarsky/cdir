#![allow(dead_code)]

use std::{any::Any, sync::Arc};

#[cfg(test)]
#[path = "event_tests.rs"]
mod event_tests;

type Payload = dyn Any + Send + Sync;

#[derive(Debug, Clone)]
pub(crate) enum ViewManagerEvent {
    Redraw,
    Resize,
    Exit(Option<String>),
}

#[derive(Debug, Clone)]
pub(crate) struct ApplicationEvent {
    pub(crate) id: String,
    pub(crate) payload: Option<Arc<Payload>>,
}

pub(crate) struct ApplicationEventBuilder {
    id: String,
    payload: Option<Arc<Payload>>,
}

impl ApplicationEventBuilder {
    pub(crate) fn new(id: &str) -> ApplicationEventBuilder {
        ApplicationEventBuilder {
            id: id.to_string(),
            payload: None,
        }
    }
    pub(crate) fn with_payload(mut self, payload: Arc<Payload>) -> ApplicationEventBuilder {
        self.payload = Some(payload);
        self
    }
    pub(crate) fn build(self) -> ApplicationEvent {
        ApplicationEvent {
            id: self.id,
            payload: self.payload,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum GenericEvent {
    ViewManagerEvent(ViewManagerEvent),
    ApplicationEvent(ApplicationEvent),
}

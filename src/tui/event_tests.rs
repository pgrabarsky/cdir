#![allow(dead_code)]

use std::sync::Arc;

use super::*;

#[test]
fn test1() {
    let payload = String::from("payload");
    let ge = GenericEvent::ApplicationEvent(
        ApplicationEventBuilder::new("id")
            .with_payload(Arc::new(payload))
            .build(),
    );
    assert!(matches!(ge, GenericEvent::ApplicationEvent(_)));
}

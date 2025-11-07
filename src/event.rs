use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub event_type: String,
    pub payload: Value,
}

impl Event {
    pub fn new(id: String, event_type: String, payload: Value) -> Self {
        Event {
            id,
            event_type,
            payload,
        }
    }
}

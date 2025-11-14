use serde_json::Value;

/// Event is simply a type alias for serde_json::Value
/// This allows the agent to accept any JSON payload without requiring specific structure
pub type Event = Value;

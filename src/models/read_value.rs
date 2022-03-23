use bevy_reflect::Uuid;
use chrono::{DateTime, Local};

pub struct ReadValue {
    pub id: Uuid,
    pub exchange_name: String,
    pub value: String,
    pub timestamp: DateTime<Local>
}

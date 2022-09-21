use serde_derive::{Deserialize, Serialize};
use bevy_reflect::{Reflect, Uuid};
use crate::models::enums::{ExchangeTypeSer, SelectedState};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ExchangeOptionsSer {
    pub exchange_name: String,
    pub exchange_type: ExchangeTypeSer,
    pub queue_routing_key: Option<String>,
    pub alias: Option<String>,
    pub pretty: Option<bool>,
    pub log_file: Option<String>,
    pub publish_file: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Reflect)]
pub struct ExchangeOptions {
    #[reflect(ignore)]
    pub id: Uuid,
    pub exchange_name: String,
    pub exchange_type: ExchangeTypeSer,
    pub queue_routing_key: String,
    pub alias: String,
    pub pretty: bool,
    pub log_file: String,
    pub publish_file: String,
    pub selected_state: SelectedState
}

impl Default for ExchangeOptions {
    fn default() -> Self {
        ExchangeOptions {
            id: Uuid::new_v4(),
            exchange_name: "".to_string(),
            exchange_type: ExchangeTypeSer::Direct,
            queue_routing_key: "".to_string(),
            alias: "".to_string(),
            pretty: false,
            log_file: "".to_string(),
            publish_file: "".to_string(),
            selected_state: SelectedState::Unselected
        }
    }
}
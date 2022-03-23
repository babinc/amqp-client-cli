use serde_derive::{Deserialize, Serialize};
use bevy_reflect::{Reflect};

#[derive(Deserialize, Serialize, Debug, Clone, Copy, Reflect)]
pub enum ExchangeTypeSer {
    /// Direct exchange; delivers messages to queues based on the routing key.
    Direct,

    /// Fanout exchange; delivers messages to all bound queues and ignores routing key.
    Fanout,

    /// Topic exchange; delivers messages based on matching between a message routing key and the
    /// pattern that was used to bind a queue to an exchange.
    Topic,

    /// Headers exchanges; ignores routing key and routes based on message header fields.
    Headers,
}

impl ExchangeTypeSer {
    pub fn iterator() -> impl Iterator<Item = ExchangeTypeSer> {
        [ExchangeTypeSer::Direct, ExchangeTypeSer::Fanout, ExchangeTypeSer::Topic, ExchangeTypeSer::Headers].iter().copied()
    }
}

impl From<&str> for ExchangeTypeSer {
    fn from(value: &str) -> Self {
        return match value {
            "Direct" => ExchangeTypeSer::Direct,
            "Fanout" => ExchangeTypeSer::Fanout,
            "Topic" => ExchangeTypeSer::Topic,
            "Headers" => ExchangeTypeSer::Headers,
            _ => ExchangeTypeSer::Direct
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Reflect, PartialEq)]
pub enum SelectedState {
    Unselected,
    PendingSubscription,
    Subscribed
}
use crate::models::Message;
use tokio::sync::broadcast;

#[derive(Debug, Clone)]
pub enum ChatEvent {
    NewMessage(Message),
}

pub struct EventBus {
    pub sender: broadcast::Sender<ChatEvent>,
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1024);
        EventBus { sender }
    }

    pub fn publish(&self, event: ChatEvent) {
        // Ignore send errors (no subscribers)
        let _ = self.sender.send(event);
    }
}

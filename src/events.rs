use crate::models::{FileInfo, Message, Reaction, RoomWithStats};
use tokio::sync::broadcast;

#[derive(Debug, Clone)]
pub enum ChatEvent {
    NewMessage(Message),
    MessageEdited(Message),
    MessageDeleted { id: String, room_id: String },
    RoomUpdated(RoomWithStats),
    Typing { sender: String, room_id: String },
    FileUploaded(FileInfo),
    FileDeleted { id: String, room_id: String },
    ReactionAdded(Reaction),
    ReactionRemoved(Reaction),
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

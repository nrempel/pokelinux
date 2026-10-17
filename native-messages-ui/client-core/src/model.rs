#![forbid(unsafe_code)]

use crate::layout::*;
use crate::protocol::{self, Page};

pub struct Model {
    pub generation: u64,
    pub revision: u64,
    pub selected: Id,
    pub cursor: Id,
    pub after: u64,
    pub status: u32,
    conversations: Vec<u8>,
    messages: Vec<u8>,
    last_conversation: Option<Id>,
    last_ordinal: Option<u64>,
}
impl Default for Model {
    fn default() -> Self {
        Self {
            generation: 0,
            revision: 0,
            selected: Id::default(),
            cursor: Id::default(),
            after: 0,
            status: UNAVAILABLE,
            conversations: b"[]".to_vec(),
            messages: b"[]".to_vec(),
            last_conversation: None,
            last_ordinal: None,
        }
    }
}
impl Model {
    fn clear_messages(&mut self) {
        self.messages = b"[]".to_vec();
        self.last_ordinal = None;
    }
    fn clear_conversations(&mut self) {
        self.conversations = b"[]".to_vec();
        self.last_conversation = None;
    }
    pub fn action(&mut self, action: u32, id: Option<Id>) -> i32 {
        match action {
            SELECT => {
                if !id.is_some_and(|v| v.valid()) {
                    return INVALID;
                }
            }
            NEXT_MESSAGES | FIRST_MESSAGES | NEXT_CONVERSATIONS | FIRST_CONVERSATIONS => {
                if id.is_some() {
                    return INVALID;
                }
            }
            _ => return INVALID,
        }
        if (action == NEXT_MESSAGES && self.last_ordinal.is_none())
            || (action == NEXT_CONVERSATIONS && self.last_conversation.is_none())
        {
            return EMPTY;
        }
        let Some(generation) = self.generation.checked_add(1) else {
            return ABORT;
        };
        let Some(revision) = self.revision.checked_add(1) else {
            return ABORT;
        };
        match action {
            SELECT => {
                if let Some(id) = id {
                    self.selected = id;
                }
                self.after = 0;
                self.clear_messages();
            }
            NEXT_MESSAGES => {
                if let Some(last) = self.last_ordinal {
                    self.after = last;
                }
                self.clear_messages();
            }
            FIRST_MESSAGES => {
                self.after = 0;
                self.clear_messages();
            }
            NEXT_CONVERSATIONS => {
                if let Some(last) = self.last_conversation {
                    self.cursor = last;
                }
                self.selected = Id::default();
                self.after = 0;
                self.clear_conversations();
                self.clear_messages();
            }
            FIRST_CONVERSATIONS => {
                self.cursor = Id::default();
                self.selected = Id::default();
                self.after = 0;
                self.clear_conversations();
                self.clear_messages();
            }
            _ => return INVALID,
        }
        self.generation = generation;
        self.revision = revision;
        OK
    }
    pub fn deliver(&mut self, bytes: &[u8]) -> i32 {
        let Some(revision) = self.revision.checked_add(1) else {
            return ABORT;
        };
        match protocol::parse(bytes) {
            Ok(Page::Conversations { raw, first, last }) => {
                self.conversations = raw;
                self.last_conversation = last;
                if self.selected.length == 0 {
                    if let Some(first) = first {
                        self.selected = first;
                        self.after = 0;
                    }
                }
                self.status = HISTORY;
            }
            Ok(Page::Messages { raw, last }) => {
                self.messages = raw;
                self.last_ordinal = last;
                self.status = HISTORY;
            }
            Err(status) => {
                self.clear_conversations();
                self.clear_messages();
                self.status = status;
            }
        }
        self.revision = revision;
        OK
    }
    pub fn view(&self) -> View {
        let mut view = View {
            generation: self.generation,
            model_revision: self.revision,
            status: self.status,
            selected: self.selected,
            conversations_after: self.cursor,
            after_ordinal: self.after,
            conversations_length: self.conversations.len() as u32,
            messages_length: self.messages.len() as u32,
            ..View::default()
        };
        view.conversations_json[..self.conversations.len()].copy_from_slice(&self.conversations);
        view.messages_json[..self.messages.len()].copy_from_slice(&self.messages);
        view
    }
}

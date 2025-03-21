use std::cell::RefCell;

use candid::{CandidType, Principal};
use ic_llm::{ChatMessage, Role};
use ic_stable_structures::{memory_manager::{MemoryId, MemoryManager, VirtualMemory}, storable::Bound, DefaultMemoryImpl, Memory, StableBTreeMap, StableCell, Storable};
use serde::{Deserialize, Serialize};

#[derive(CandidType, Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum Roles {
    #[serde(rename = "system")]
    System,
    #[serde(rename = "user")]
    User,
    #[serde(rename = "assistant")]
    Assistant,
}

pub type MessageId = u64;

#[derive(CandidType, Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct Message {
    pub id: MessageId,
    pub conversation: u64,
    pub content: String,
    pub timestamp: u64,
    pub role: Roles,
}

pub type ConversationId = u64;

#[derive(CandidType, Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct Conversation {
    pub id: ConversationId,
    pub user: u64,
    pub updated_at: u64,
    pub name: String,
}

pub type UserId = u64;

#[derive(CandidType, Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct User {
    pub id: UserId,
    pub username: String,
    pub identity: Principal,
}

impl Storable for Message {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        let mut encoded = Vec::new();
        ciborium::into_writer(self, &mut encoded).unwrap();
        std::borrow::Cow::Owned(encoded)
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        ciborium::from_reader(bytes.as_ref()).unwrap()
    }
    const BOUND: Bound = Bound::Unbounded;
}

impl Storable for Conversation {

    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        let mut encoded = Vec::new();
        ciborium::into_writer(self, &mut encoded).unwrap();
        std::borrow::Cow::Owned(encoded)
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        ciborium::from_reader(bytes.as_ref()).unwrap()
    }
    const BOUND: Bound = Bound::Unbounded;
}

impl Storable for User {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        let mut encoded = Vec::new();
        ciborium::into_writer(self, &mut encoded).unwrap();
        std::borrow::Cow::Owned(encoded)
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        ciborium::from_reader(bytes.as_ref()).unwrap()
    }
    const BOUND: Bound = Bound::Unbounded;
}

impl Roles {
    pub fn to_ic_role(&self) -> Role {
        match *self {
            Roles::Assistant => Role::Assistant,
            Roles::User => Role::User,
            Roles::System => Role::System,
        }
    }
}

impl Message {
    pub fn to_ic_message(&self) -> ChatMessage {
        ChatMessage { role: self.role.to_ic_role(), content: self.content.clone() }
    }
}

type Memo = VirtualMemory<DefaultMemoryImpl>;
type BigSerialCell = RefCell<StableCell<u64, Memo>>;
type BTreeMapCell<K,V> = RefCell<StableBTreeMap<K, V, Memo>>;

const SERIAL_CHAT_MESSAGE_MEMORY_ID: MemoryId = MemoryId::new(0);
const SERIAL_CONVERSATION_MEMORY_ID: MemoryId = MemoryId::new(1);
const CHAT_MESSAGE_MEMORY_ID: MemoryId = MemoryId::new(2);
const CONVERSATION_MEMORY_ID: MemoryId = MemoryId::new(3);
const USER_MEMORY_ID: MemoryId = MemoryId::new(4);

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );

    static NEXT_CHAT_MESSAGE_ID: BigSerialCell = RefCell::new(
        StableCell::init(
            MEMORY_MANAGER.with_borrow(|m| m.get(SERIAL_CHAT_MESSAGE_MEMORY_ID)), 1
        ).expect("failed to init NEXT_CHAT_MESSAGE_ID")
    );

    static NEXT_CONVERSATION_ID: BigSerialCell = RefCell::new(
        StableCell::init(
            MEMORY_MANAGER.with_borrow(|m| m.get(SERIAL_CONVERSATION_MEMORY_ID)), 1
        ).expect("failed to init NEXT_CONVERSATION_ID")
    );

    static CHAT_MESSAGE: BTreeMapCell<(ConversationId, MessageId), Message> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with_borrow(|m| m.get(CHAT_MESSAGE_MEMORY_ID))
        )
    );

    static CONVERSATION: BTreeMapCell<(UserId, ConversationId), Conversation> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with_borrow(|m| m.get(CONVERSATION_MEMORY_ID))
        )
    );

    static USER: BTreeMapCell<u64, User> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with_borrow(|m| m.get(USER_MEMORY_ID))
        )
    );
}

pub trait SerialId<M>
where M: Memory {
    fn with_generator<F, R>(f: F) -> R
    where
        F: FnOnce(&mut StableCell<u64, M>) -> R;

    fn peek_next_id(&self) -> u64 {
        Self::with_generator(|v| *v.get())
    }

    fn next_id(&self) -> u64 {
        Self::with_generator(|v| {
            let inc = *v.get() + 1;
            v.set(inc).unwrap();
            inc
        })
    }
}

#[derive(Default, Debug)]
pub struct MessageRepository;

impl SerialId<Memo> for MessageRepository {
    fn with_generator<F, R>(f: F) -> R
    where
        F: FnOnce(&mut StableCell<u64, Memo>) -> R {
        NEXT_CHAT_MESSAGE_ID.with_borrow_mut(|m| f(m))
    }
}

impl MessageRepository {
    fn insert(&self, value: Message) -> Option<Message> {
        let key = (value.conversation, value.id);
        CHAT_MESSAGE.with_borrow_mut(|m| m.insert(key, value))
    }

    fn paged_list(&self, conversation: ConversationId, cursor: Option<MessageId>, limit: usize) -> Vec<Message> {
        let end = cursor.map_or(MessageId::MAX, |c| c.saturating_sub(1));
        let range = (conversation, 0)..=(conversation, end);

        CHAT_MESSAGE.with_borrow(|m| m.range(range)
            .rev()
            .take(limit)
            .map(|(_, msg)| msg)
            .collect())
    }
}

#[derive(Default, Debug)]
pub struct ConversationRepository;

impl SerialId<Memo> for ConversationRepository {
    fn with_generator<F, R>(f: F) -> R
    where
        F: FnOnce(&mut StableCell<u64, Memo>) -> R {
        NEXT_CONVERSATION_ID.with_borrow_mut(|m| f(m))
    }
}

impl ConversationRepository {
    // fn get(&self, key: Option<ConversationId>) -> Option<Conversation> {
    //     let id = match key {
    //         None => return None,
    //         Some(id) => id,
    //     };
    //     CONVERSATION.with_borrow(|m| )
    // }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_message_should_valid() {
        let m = Message {
            id: 1,
            conversation: 1,
            content: "hi text!".to_string(),
            timestamp: 1,
            role: Roles::User,
        };
        let mapped = m.to_ic_message();
        assert_eq!(m.content, mapped.content);
    }

    #[test]
    fn map_role_should_valid() {
        let r = Roles::Assistant;
        let mapped = r.to_ic_role();
        assert!(matches!(mapped, Role::Assistant));
        let r = Roles::System;
        let mapped = r.to_ic_role();
        assert!(matches!(mapped, Role::System));
    }

    #[test]
    fn storable_encoding_decoding_should_valid() {
        let message = Message {
            id: 1,
            conversation: 1,
            content: "Hello, world!".to_string(),
            timestamp: 1234567890,
            role: Roles::User,
        };
        let encoded_message = message.to_bytes();
        let decoded_message = Message::from_bytes(encoded_message);
        assert_eq!(message, decoded_message);

        let conversation = Conversation {
            id: 1,
            user: 1,
            updated_at: 1234567890,
            name: "Test Conversation".to_string(),
        };
        let encoded_conversation = conversation.to_bytes();
        let decoded_conversation = Conversation::from_bytes(encoded_conversation);
        assert_eq!(conversation, decoded_conversation);

        let user = User {
            id: 1,
            username: "test_user".to_string(),
            identity: Principal::anonymous(),
        };
        let encoded_user = user.to_bytes();
        let decoded_user = User::from_bytes(encoded_user);
        assert_eq!(user, decoded_user);
    }

    #[test]
    fn generated_id_should_consistent() {
        let msg_repo = MessageRepository;
        let con_repo = ConversationRepository;

        assert_eq!(msg_repo.peek_next_id(), 1);
        assert_eq!(con_repo.peek_next_id(), 1);

        assert_eq!(msg_repo.next_id(), 2);
        assert_eq!(msg_repo.peek_next_id(), 2);
        assert_eq!(con_repo.next_id(), 2);
        assert_eq!(con_repo.peek_next_id(), 2);
    }

    #[test]
    fn message_cursor_paged_list_should_return_correct_list() {
        let repo = MessageRepository;

        // Insert 10 messages (IDs 1..=10)
        for i in 1..=10 {
            CHAT_MESSAGE.with(|m| {
                m.borrow_mut().insert((1, i), Message {
                    id: i,
                    conversation: 1,
                    content: format!("Message {}", i),
                    timestamp: i,
                    role: Roles::User,
                });
            });
        }

        // Initial load (latest 3 messages)
        let page1 = repo.paged_list(1, None, 3);
        assert_eq!(page1.iter().map(|m| m.id).collect::<Vec<_>>(), vec![10, 9, 8]);

        // Scroll up (older than 8)
        let page2 = repo.paged_list(1, Some(8), 3);
        assert_eq!(page2.iter().map(|m| m.id).collect::<Vec<_>>(), vec![7, 6, 5]);
    }
}

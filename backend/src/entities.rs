use std::cell::RefCell;

use candid::{CandidType, Principal};
use ic_llm::{ChatMessage, Role};
use ic_stable_structures::{memory_manager::{MemoryId, MemoryManager, VirtualMemory}, storable::Bound, DefaultMemoryImpl, StableBTreeMap, StableCell, Storable};
use serde::{Deserialize, Serialize};

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub enum Roles {
    #[serde(rename = "system")]
    System,
    #[serde(rename = "user")]
    User,
    #[serde(rename = "assistant")]
    Assistant,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct Message {
    pub id: u64,
    pub conversation: u64,
    pub content: String,
    pub timestamp: u64,
    pub role: Roles,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct Conversation {
    pub id: u64,
    pub user: u64,
    pub updated_at: u64,
    pub name: String,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct User {
    pub id: u64,
    pub username: String,
    pub identity: Principal,
}

pub type CompositeKey = (u64, u64);

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

    static CHAT_MESSAGE: BTreeMapCell<CompositeKey, Message> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with_borrow(|m| m.get(CHAT_MESSAGE_MEMORY_ID))
        )
    );

    static CONVERSATION: BTreeMapCell<CompositeKey, Conversation> = RefCell::new(
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

pub trait Repository<K, E>
where K: Eq + Clone + Ord + Storable, E : Storable {
    fn get(&self, key: &K) -> Option<E>;
    fn insert(&self, key: &K, value: E);
    fn list(&self, cursor: Option<K>, limit: usize) -> Vec<Message>;
}

#[derive(Default, Debug)]
pub struct MessageRepository;

// impl Repository<CompositeKey, Message> for MessageRepository {
//     fn get(&self, key: &CompositeKey) -> Option<Message> {
//         // CHAT_MESSAGE.with_borrow(|m| {

//         // })
//     }

//     fn insert(&self, key: &CompositeKey, value: Message) {
//         todo!()
//     }

//     fn list(&self, cursor: Option<CompositeKey>, limit: usize) -> Vec<Message> {
//         todo!()
//     }
// }

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
    }
}

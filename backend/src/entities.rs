use std::{cell::RefCell, cmp::Reverse};

use bitcode::{Decode, Encode};
use candid::{CandidType, Principal};
use ic_llm::{ChatMessage, Role};
use ic_stable_structures::{
    memory_manager::{MemoryId, MemoryManager, VirtualMemory},
    storable::Bound,
    DefaultMemoryImpl, Memory, StableBTreeMap, StableCell, Storable,
};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[cfg(all(test, not(rust_analyzer)))]
use crate::utils::mock_timestamp::timestamp;
#[cfg(any(not(test), rust_analyzer))]
use crate::utils::timestamp;

/// Represents a timestamp in the system.
pub type Timestamp = u64;

/// Enum representing different roles in the system.
#[derive(CandidType, Serialize, Deserialize, Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub enum Roles {
    #[serde(rename = "system")]
    System,
    #[serde(rename = "user")]
    User,
    #[serde(rename = "assistant")]
    Assistant,
}

/// Represents a unique identifier for a message.
pub type MessageId = u64;

/// Struct representing a message in a conversation.
#[derive(CandidType, Serialize, Deserialize, Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub struct Message {
    pub id: MessageId,
    pub conversation: u64,
    pub content: String,
    pub timestamp: Timestamp,
    pub role: Roles,
}

/// Represents a unique identifier for a conversation.
pub type ConversationId = u64;

/// Struct representing a conversation between users.
#[derive(CandidType, Serialize, Deserialize, Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub struct Conversation {
    pub id: ConversationId,
    pub user: u64,
    pub updated_at: Timestamp,
    pub name: String,
}

/// Represents a unique identifier for a user.
pub type UserId = u64;

/// Struct representing a user in the system.
#[derive(CandidType, Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct User {
    pub id: UserId,
    pub username: String,
    pub identity: Principal,
    pub resume: String,
}

impl Storable for Message {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        std::borrow::Cow::Owned(bitcode::encode(self))
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        bitcode::decode(bytes.as_ref()).unwrap()
    }
    const BOUND: Bound = Bound::Unbounded;
}

impl Storable for Conversation {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        std::borrow::Cow::Owned(bitcode::encode(self))
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        bitcode::decode(bytes.as_ref()).unwrap()
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
    /// Converts a `Roles` enum to an `ic_llm::Role`.
    pub fn to_ic_role(&self) -> Role {
        match *self {
            Roles::Assistant => Role::Assistant,
            Roles::User => Role::User,
            Roles::System => Role::System,
        }
    }
}

impl Message {
    /// Converts a `Message` struct to an `ic_llm::ChatMessage`.
    pub fn to_ic_message(&self) -> ChatMessage {
        ChatMessage {
            role: self.role.to_ic_role(),
            content: self.content.clone(),
        }
    }
}

type Memo = VirtualMemory<DefaultMemoryImpl>;
type BigSerialCell = RefCell<StableCell<u64, Memo>>;
type BTreeMapCell<K, V> = RefCell<StableBTreeMap<K, V, Memo>>;
type ConversationIndex = (UserId, Reverse<Timestamp>, ConversationId);

const SERIAL_CHAT_MESSAGE_MEMORY_ID: MemoryId = MemoryId::new(0);
const SERIAL_CONVERSATION_MEMORY_ID: MemoryId = MemoryId::new(1);
const CHAT_MESSAGE_MEMORY_ID: MemoryId = MemoryId::new(2);
const CONVERSATION_MEMORY_ID: MemoryId = MemoryId::new(3);
const USER_MEMORY_ID: MemoryId = MemoryId::new(4);
const CHAT_MESSAGE_CONVERSATION_INDEX_MEMORY_ID: MemoryId = MemoryId::new(5);
const CONVERSATION_UPDATED_INDEX_MEMORY_ID: MemoryId = MemoryId::new(6);

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

    static CHAT_MESSAGE: BTreeMapCell<Reverse<MessageId>, Message> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with_borrow(|m| m.get(CHAT_MESSAGE_MEMORY_ID))
        )
    );

    static CONVERSATION: BTreeMapCell<ConversationId, Conversation> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with_borrow(|m| m.get(CONVERSATION_MEMORY_ID))
        )
    );

    static USER: BTreeMapCell<UserId, User> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with_borrow(|m| m.get(USER_MEMORY_ID))
        )
    );

    static CHAT_MESSAGE_CONVERSATION_INDEX: BTreeMapCell<(ConversationId, Reverse<MessageId>), ()> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with_borrow(|m| m.get(CHAT_MESSAGE_CONVERSATION_INDEX_MEMORY_ID))
        )
    );

    static CONVERSATION_USER_INDEX: BTreeMapCell<ConversationIndex, ()> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with_borrow(|m| m.get(CONVERSATION_UPDATED_INDEX_MEMORY_ID))
        )
    );
}

#[derive(Error, Debug, Eq, PartialEq, Clone)]
pub enum RepositoryError {
    #[error(r#"The requested entity was not found in the repository."#)]
    NotFound,
    #[error(r#"Cannot write on existing entity."#)]
    Conflict,
    #[error(r#"Invalid update operation: {reason}."#)]
    IllegalUpdate { reason: String },
}

pub type RepositoryResult<T> = Result<T, RepositoryError>;

pub trait Repository<K, V>
where
    K: Clone + Ord + Storable,
    V: Clone + Storable,
{
    fn get(&self, id: &K) -> Option<V>;
    fn insert(&self, value: V) -> RepositoryResult<V>;
    fn update(&self, value: V) -> RepositoryResult<V>;
}

pub trait SerialIdRepository<M>
where
    M: Memory,
{
    /// Wrap Serial Id data structure
    fn with_generator<F, R>(f: F) -> R
    where
        F: FnOnce(&mut StableCell<u64, M>) -> R;

    /// Peek the next value for the serial id
    fn peek_next_id(&self) -> u64 {
        Self::with_generator(|v| *v.get())
    }

    /// Get the next id and increment
    fn next_id(&self) -> u64 {
        Self::with_generator(|v| {
            let id = *v.get();
            v.set(id + 1).unwrap();
            id
        })
    }
}

pub trait IndexedRepository<V>
where
    V: Clone + Storable,
{
    /// Removes the indexes for the current value
    fn remove_indexes(&self, value: &V);

    /// Adds the indexes for the current value
    fn add_indexes(&self, value: &V);

    /// Clears all the indexes
    fn clear_indexes(&self);

    /// Saves the indexes for the current value and removes the old indexes if
    /// the value has changed.
    fn save_indexes(&self, value: &V, old_value: Option<&V>) {
        if let Some(existing) = old_value {
            self.remove_indexes(existing);
        }
        self.add_indexes(value);
    }
}

pub trait IndexManagementRepository<I, T> {
    type Criteria;
    type Cursor;

    /// Checks if an index exists.
    fn exists(&self, index: &I) -> bool;

    /// Inserts a new index.
    fn insert(&self, index: I);

    /// Removes an index.
    fn remove(&self, index: &I) -> bool;

    /// Clears all indexes.
    fn clear(&self);

    /// Finds entities based on criteria and cursor with a limit.
    fn find(&self, criteria: Self::Criteria, cursor: Option<Self::Cursor>, limit: usize) -> Vec<T>;
}

#[derive(Default, Debug)]
pub struct MessageConversationIndexRepository;

#[derive(Default, Debug)]
pub struct MessageRepository {
    pub conversation_index: MessageConversationIndexRepository,
}

impl IndexManagementRepository<(ConversationId, Reverse<MessageId>), MessageId>
    for MessageConversationIndexRepository
{
    type Criteria = ConversationId;
    type Cursor = MessageId;

    fn exists(&self, index: &(ConversationId, Reverse<MessageId>)) -> bool {
        CHAT_MESSAGE_CONVERSATION_INDEX.with_borrow(|m| m.get(index).is_some())
    }

    fn insert(&self, index: (ConversationId, Reverse<MessageId>)) {
        CHAT_MESSAGE_CONVERSATION_INDEX.with_borrow_mut(|m| m.insert(index, ()));
    }

    fn remove(&self, index: &(ConversationId, Reverse<MessageId>)) -> bool {
        CHAT_MESSAGE_CONVERSATION_INDEX.with_borrow_mut(|m| m.remove(index).is_some())
    }

    fn clear(&self) {
        CHAT_MESSAGE_CONVERSATION_INDEX.with_borrow_mut(|m| m.clear_new());
    }

    fn find(
        &self,
        conversation: Self::Criteria,
        cursor: Option<Self::Cursor>,
        limit: usize,
    ) -> Vec<MessageId> {
        let last_id = cursor.map_or(MessageId::MAX, |c| c.saturating_sub(1));
        let start = (conversation, Reverse(last_id));
        let end = (conversation, Reverse(1));
        CHAT_MESSAGE_CONVERSATION_INDEX.with_borrow(|m| {
            m.range(start..=end)
                .take(limit)
                .map(|((_, id), _)| id.0)
                .collect_vec()
        })
    }
}

impl IndexedRepository<Message> for MessageRepository {
    fn remove_indexes(&self, value: &Message) {
        self.conversation_index
            .remove(&(value.conversation, Reverse(value.id)));
    }

    fn add_indexes(&self, value: &Message) {
        self.conversation_index
            .insert((value.conversation, Reverse(value.id)));
    }

    fn clear_indexes(&self) {
        self.conversation_index.clear();
    }
}

impl SerialIdRepository<Memo> for MessageRepository {
    fn with_generator<F, R>(f: F) -> R
    where
        F: FnOnce(&mut StableCell<u64, Memo>) -> R,
    {
        NEXT_CHAT_MESSAGE_ID.with_borrow_mut(|m| f(m))
    }
}

impl Repository<MessageId, Message> for MessageRepository {
    /// Retrieves a message by its ID.
    fn get(&self, id: &MessageId) -> Option<Message> {
        CHAT_MESSAGE.with_borrow(|m| m.get(&Reverse(*id)))
    }

    /// Inserts a new message into the repository.
    fn insert(&self, mut msg: Message) -> RepositoryResult<Message> {
        msg.id = self.next_id();
        msg.timestamp = timestamp();
        let prev = CHAT_MESSAGE.with_borrow_mut(|m| m.insert(Reverse(msg.id), msg.clone()));
        self.save_indexes(&msg, prev.as_ref());
        Ok(msg)
    }

    /// Update will always Error, because message is immutable on current design.
    fn update(&self, value: Message) -> RepositoryResult<Message> {
        Err(RepositoryError::IllegalUpdate {
            reason: "Message entity cannot be updated".to_string(),
        })
    }
}

impl MessageRepository {
    /// Retrieves a paginated list of messages for a conversation.
    pub fn paged_list(
        &self,
        conversation: ConversationId,
        cursor: Option<MessageId>,
        limit: usize,
    ) -> (Option<MessageId>, Vec<Message>) {
        let messages = self
            .conversation_index
            .find(conversation, cursor, limit)
            .iter()
            .filter_map(|id| self.get(id))
            .collect_vec();
        (messages.last().map(|m| m.id), messages)
    }
}

#[derive(Default, Debug)]
pub struct ConversationUserIndexRepository;

#[derive(Default, Debug)]
pub struct ConversationRepository {
    pub user_index: ConversationUserIndexRepository,
}

impl IndexManagementRepository<ConversationIndex, ConversationId>
    for ConversationUserIndexRepository
{
    type Criteria = UserId;
    type Cursor = Timestamp;

    fn exists(&self, index: &ConversationIndex) -> bool {
        CONVERSATION_USER_INDEX.with_borrow(|m| m.get(index).is_some())
    }

    fn insert(&self, index: ConversationIndex) {
        CONVERSATION_USER_INDEX.with_borrow_mut(|m| m.insert(index, ()));
    }

    fn remove(&self, index: &ConversationIndex) -> bool {
        CONVERSATION_USER_INDEX.with_borrow_mut(|m| m.remove(index).is_some())
    }

    fn clear(&self) {
        CONVERSATION_USER_INDEX.with_borrow_mut(|m| m.clear_new());
    }

    fn find(
        &self,
        user_id: Self::Criteria,
        cursor: Option<Timestamp>,
        limit: usize,
    ) -> Vec<ConversationId> {
        let ts = cursor.map_or(Timestamp::MAX, |ts| ts - 1);
        let start = (user_id, Reverse(ts), 0);
        let end = (user_id, Reverse(0), ConversationId::MAX);

        if limit == 0 {
            CONVERSATION_USER_INDEX
                .with_borrow(|m| m.range(start..=end).map(|((_, _, c_id), _)| c_id).collect())
        } else {
            CONVERSATION_USER_INDEX.with_borrow(|m| {
                m.range(start..=end)
                    .take(limit)
                    .map(|((_, _, c_id), _)| c_id)
                    .collect()
            })
        }
    }
}

impl IndexedRepository<Conversation> for ConversationRepository {
    fn remove_indexes(&self, conv: &Conversation) {
        self.user_index
            .remove(&(conv.user, Reverse(conv.updated_at), conv.id));
    }

    fn add_indexes(&self, conv: &Conversation) {
        self.user_index
            .insert((conv.user, Reverse(conv.updated_at), conv.id));
    }

    fn clear_indexes(&self) {
        self.user_index.clear();
    }
}

impl SerialIdRepository<Memo> for ConversationRepository {
    fn with_generator<F, R>(f: F) -> R
    where
        F: FnOnce(&mut StableCell<u64, Memo>) -> R,
    {
        NEXT_CONVERSATION_ID.with_borrow_mut(|m| f(m))
    }
}

impl Repository<ConversationId, Conversation> for ConversationRepository {
    /// Retrieves a conversation by its ID.
    fn get(&self, id: &ConversationId) -> Option<Conversation> {
        CONVERSATION.with_borrow(|m| m.get(id))
    }

    /// Inserts a new conversation into the repository.
    fn insert(&self, mut conversation: Conversation) -> RepositoryResult<Conversation> {
        conversation.id = self.next_id();
        conversation.updated_at = timestamp();
        let prev =
            CONVERSATION.with_borrow_mut(|m| m.insert(conversation.id, conversation.clone()));
        self.save_indexes(&conversation, prev.as_ref());

        Ok(conversation)
    }

    /// Update the conversation on the repository.
    fn update(&self, mut conversation: Conversation) -> RepositoryResult<Conversation> {
        if let Some(old_conv) = self.get(&conversation.id) {
            if old_conv.user != conversation.user {
                return Err(RepositoryError::IllegalUpdate {
                    reason: "User is different".to_string(),
                });
            }
        } else {
            return Err(RepositoryError::NotFound);
        }
        conversation.updated_at = timestamp();
        let prev =
            CONVERSATION.with_borrow_mut(|m| m.insert(conversation.id, conversation.clone()));
        self.save_indexes(&conversation, prev.as_ref());

        Ok(conversation)
    }
}

impl ConversationRepository {
    /// Inserts or updates a conversation in the repository.
    pub fn upsert(&self, conversation: Conversation) -> RepositoryResult<Conversation> {
        match self.get(&conversation.id) {
            Some(_) => self.update(conversation),
            None => self.insert(conversation),
        }
    }

    /// Retrieves a paginated list of conversations for a user. Cursor is using Timestamp instead of id
    pub fn paged_list(
        &self,
        user_id: UserId,
        cursor: Option<Timestamp>,
        limit: usize,
    ) -> (Option<Timestamp>, Vec<Conversation>) {
        let conv = self
            .user_index
            .find(user_id, cursor, limit)
            .iter()
            .filter_map(|id| self.get(id))
            .collect_vec();
        (conv.last().map(|m| m.id), conv)
    }
}

pub struct UserRepository;

#[cfg(test)]
mod tests {
    use crate::utils::mock_timestamp;

    use super::*;

    fn reset_msg_data() {
        CHAT_MESSAGE.with_borrow_mut(|m| m.clear_new());
        CHAT_MESSAGE_CONVERSATION_INDEX.with_borrow_mut(|m| m.clear_new());
        NEXT_CHAT_MESSAGE_ID.with_borrow_mut(|v| v.set(1).unwrap());
    }

    fn reset_conv_data() {
        CONVERSATION.with_borrow_mut(|m| m.clear_new());
        CONVERSATION_USER_INDEX.with_borrow_mut(|m| m.clear_new());
        NEXT_CONVERSATION_ID.with_borrow_mut(|v| v.set(1).unwrap());
    }

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
            resume: "engineer".to_string(),
        };
        let encoded_user = user.to_bytes();
        let decoded_user = User::from_bytes(encoded_user);
        assert_eq!(user, decoded_user);
    }

    #[test]
    fn generated_id_should_consistent() {
        reset_msg_data();
        reset_conv_data();
        let msg_repo = MessageRepository::default();
        let con_repo = ConversationRepository::default();

        assert_eq!(msg_repo.peek_next_id(), 1);
        assert_eq!(con_repo.peek_next_id(), 1);

        assert_eq!(msg_repo.next_id(), 1);
        assert_eq!(msg_repo.peek_next_id(), 2);
        assert_eq!(con_repo.next_id(), 1);
        assert_eq!(con_repo.peek_next_id(), 2);
        assert_eq!(con_repo.next_id(), 2);
        assert_eq!(con_repo.peek_next_id(), 3);
    }

    #[test]
    fn get_and_insert_message_should_work() {
        reset_msg_data();
        let repo = MessageRepository::default();
        repo.insert(Message {
            id: 123,
            conversation: 1,
            content: "Hi World!".to_string(),
            timestamp: 123,
            role: Roles::User,
        })
        .unwrap();
        assert!(repo.get(&123).is_none());
        assert!(repo.get(&1).is_some());
        assert_eq!("Hi World!".to_string(), repo.get(&1).unwrap().content)
    }

    #[test]
    #[should_panic]
    fn update_message_should_failed() {
        reset_msg_data();
        let repo = MessageRepository::default();
        repo.update(Message {
            id: 1,
            conversation: 1,
            content: "one".to_string(),
            timestamp: 0,
            role: Roles::Assistant,
        })
        .unwrap();
    }

    #[test]
    fn message_cursor_paged_list_should_return_correct_list() {
        reset_msg_data();
        let repo = MessageRepository::default();

        // 1-5 for conv 1
        for i in 1..=5 {
            repo.insert(Message {
                id: i,
                conversation: 1,
                content: format!("Message {}", i),
                timestamp: i,
                role: Roles::User,
            })
            .unwrap();
        }
        // 6-9 for conv 2
        for i in 6..=9 {
            repo.insert(Message {
                id: i,
                conversation: 2,
                content: format!("Message {}", i),
                timestamp: 2,
                role: Roles::User,
            })
            .unwrap();
        }
        // 10 for conv 1
        repo.insert(Message {
            id: 10,
            conversation: 1,
            content: format!("Message {}", 10),
            timestamp: 10,
            role: Roles::User,
        })
        .unwrap();

        // Initial load (latest 3 messages for conv 1)
        let (next_cursor, page1) = repo.paged_list(1, None, 3);
        assert_eq!(
            page1.iter().map(|m| m.id).collect::<Vec<_>>(),
            vec![10, 5, 4]
        );
        assert_eq!(next_cursor.unwrap(), 4);

        // Scroll up (older than 10)
        let (_, page2) = repo.paged_list(1, Some(10), 3);
        assert_eq!(
            page2.iter().map(|m| m.id).collect::<Vec<_>>(),
            vec![5, 4, 3]
        );

        // conv 2 out of limit
        let (_, conv2) = repo.paged_list(2, Some(8), 5);
        assert_eq!(conv2.iter().map(|m| m.id).collect::<Vec<_>>(), vec![7, 6]);
    }

    #[test]
    fn get_and_upsert_conversation_should_work() {
        reset_conv_data();
        let repo = ConversationRepository::default();
        let conversation = Conversation {
            id: 1,
            user: 1,
            updated_at: 1234567890,
            name: "Test Conversation".to_string(),
        };
        repo.upsert(conversation.clone()).unwrap();
        assert!(repo.get(&1).is_some());
        assert_eq!("Test Conversation".to_string(), repo.get(&1).unwrap().name);
    }

    #[test]
    fn conversation_cursor_paged_list_should_return_correct_list() {
        reset_conv_data();
        mock_timestamp::reset_to(1);
        let repo = ConversationRepository::default();

        // 1-5 for user 1
        for i in 1..=5 {
            repo.upsert(Conversation {
                id: i,
                user: 1,
                updated_at: i,
                name: format!("Conversation {}", i),
            })
            .unwrap();
        }
        // 6-9 for user 2
        for i in 6..=9 {
            repo.upsert(Conversation {
                id: i,
                user: 2,
                updated_at: i,
                name: format!("Conversation {}", i),
            })
            .unwrap();
        }
        // 10 for user 1
        repo.upsert(Conversation {
            id: 10,
            user: 1,
            updated_at: 10,
            name: format!("Conversation {}", 10),
        })
        .unwrap();

        // Initial load (latest 3 conversations for user 1)
        let (next_cursor, page1) = repo.paged_list(1, None, 3);
        assert_eq!(
            page1.iter().map(|c| c.id).collect::<Vec<_>>(),
            vec![10, 5, 4]
        );
        assert_eq!(next_cursor.unwrap(), 4);

        // Scroll up (older than 10)
        let (_, page2) = repo.paged_list(1, Some(10), 3);
        assert_eq!(
            page2.iter().map(|c| c.id).collect::<Vec<_>>(),
            vec![5, 4, 3]
        );

        // user 2 out of limit
        let (_, user2) = repo.paged_list(2, Some(8), 5);
        assert_eq!(user2.iter().map(|c| c.id).collect::<Vec<_>>(), vec![7, 6]);
    }
}

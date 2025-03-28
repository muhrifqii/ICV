use std::{cell::RefCell, cmp::Reverse, str::FromStr, sync::Arc};

use bitcode::{Decode, Encode};
use candid::{CandidType, Principal};
use ic_llm::{ChatMessage, Role};
use ic_stable_structures::{
    memory_manager::{MemoryId, MemoryManager, VirtualMemory},
    storable::Bound,
    DefaultMemoryImpl, Memory, StableBTreeMap, StableCell, Storable,
};
use itertools::Itertools;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[cfg(all(test, not(rust_analyzer)))]
use crate::utils::mock_ic0::timestamp;
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
    pub fullname: String,
    pub identity: Principal,
    pub resume: String,
}

#[derive(Error, Debug, Eq, PartialEq, Clone)]
pub enum EntityError {
    #[error(r#"Such role does not exist."#)]
    UnknownRoles,
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

impl FromStr for Roles {
    type Err = EntityError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let r = match s {
            "assistant" => Self::Assistant,
            "user" => Self::User,
            "system" => Self::System,
            _ => return Err(EntityError::UnknownRoles),
        };
        Ok(r)
    }
}

impl Roles {
    /// Converts a `Roles` to an `ic_llm::Role`.
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
const USER_PRINCIPAL_INDEX_MEMORY_ID: MemoryId = MemoryId::new(7);
const SERIAL_USER_MEMORY_ID: MemoryId = MemoryId::new(8);

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

    static USER_PRINCIPAL_INDEX: BTreeMapCell<(Principal, UserId), ()> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with_borrow(|m|m.get(USER_PRINCIPAL_INDEX_MEMORY_ID))
        )
    );

    static NEXT_USER_ID: BigSerialCell = RefCell::new(
        StableCell::init(
            MEMORY_MANAGER.with_borrow(|m| m.get(SERIAL_USER_MEMORY_ID)), 1
        ).expect("failed to init NEXT_USER_ID")
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
    fn delete(&self, id: &K) -> RepositoryResult<K>;
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
            v.set(id + 1).unwrap()
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
        if limit == usize::default() {
            CHAT_MESSAGE_CONVERSATION_INDEX
                .with_borrow(|m| m.range(start..=end).map(|((_, id), _)| id.0).collect_vec())
        } else {
            CHAT_MESSAGE_CONVERSATION_INDEX.with_borrow(|m| {
                m.range(start..=end)
                    .take(limit)
                    .map(|((_, id), _)| id.0)
                    .collect_vec()
            })
        }
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

    /// delete message by id, if such id does not exist, return NotFound error.
    fn delete(&self, id: &MessageId) -> RepositoryResult<MessageId> {
        let old = CHAT_MESSAGE.with_borrow_mut(|m| m.remove(&Reverse(*id)));
        if old.is_none() {
            Err(RepositoryError::NotFound)
        } else {
            self.remove_indexes(&old.unwrap());
            Ok(*id)
        }
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

    pub fn delete_by_conversation(
        &self,
        conversation: &ConversationId,
    ) -> RepositoryResult<Vec<MessageId>> {
        let deleted_ids = self
            .conversation_index
            .find(*conversation, None, 0)
            .iter()
            .filter_map(|id| self.delete(id).ok())
            .collect_vec();
        Ok(deleted_ids)
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

        if limit == usize::default() {
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

    fn delete(&self, id: &ConversationId) -> RepositoryResult<ConversationId> {
        let old = CONVERSATION.with_borrow_mut(|m| m.remove(id));
        if old.is_none() {
            Err(RepositoryError::NotFound)
        } else {
            self.remove_indexes(&old.unwrap());
            Ok(*id)
        }
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

#[derive(Debug, Default)]
pub struct UserIdentityIndexRepository;

#[derive(Debug, Default)]
pub struct UserRepository {
    identity_index: UserIdentityIndexRepository,
}

pub trait IdentityProvider {
    fn get_user(&self, identity: Principal) -> Option<User>;
}

impl IndexManagementRepository<(Principal, UserId), UserId> for UserIdentityIndexRepository {
    type Criteria = Principal;
    type Cursor = UserId;

    fn exists(&self, index: &(Principal, UserId)) -> bool {
        USER_PRINCIPAL_INDEX.with_borrow(|m| m.get(index).is_some())
    }

    fn insert(&self, index: (Principal, UserId)) {
        USER_PRINCIPAL_INDEX.with_borrow_mut(|m| m.insert(index, ()));
    }

    fn remove(&self, index: &(Principal, UserId)) -> bool {
        USER_PRINCIPAL_INDEX.with_borrow_mut(|m| m.remove(index).is_some())
    }

    fn clear(&self) {
        USER_PRINCIPAL_INDEX.with_borrow_mut(|m| m.clear_new());
    }

    fn find(
        &self,
        principal: Self::Criteria,
        _cursor: Option<Self::Cursor>,
        _limit: usize,
    ) -> Vec<UserId> {
        let start = (principal, 1);
        let end = (principal, UserId::MAX);
        USER_PRINCIPAL_INDEX
            .with_borrow(|m| m.range(start..=end).map(|((_, id), _)| id).collect_vec())
    }
}

impl IndexedRepository<User> for UserRepository {
    fn remove_indexes(&self, value: &User) {
        self.identity_index.remove(&(value.identity, value.id));
    }

    fn add_indexes(&self, value: &User) {
        self.identity_index.insert((value.identity, value.id));
    }

    fn clear_indexes(&self) {
        self.identity_index.clear();
    }
}

impl SerialIdRepository<Memo> for UserRepository {
    fn with_generator<F, R>(f: F) -> R
    where
        F: FnOnce(&mut StableCell<u64, Memo>) -> R,
    {
        NEXT_USER_ID.with_borrow_mut(|m| f(m))
    }
}

impl Repository<UserId, User> for UserRepository {
    fn get(&self, id: &UserId) -> Option<User> {
        USER.with_borrow(|m| m.get(id))
    }

    fn insert(&self, mut user: User) -> RepositoryResult<User> {
        user.id = self.next_id();
        let prev = USER.with_borrow_mut(|m| m.insert(user.id, user.clone()));
        self.save_indexes(&user, prev.as_ref());
        Ok(user)
    }

    fn update(&self, user: User) -> RepositoryResult<User> {
        if self.get(&user.id).is_none() {
            return Err(RepositoryError::NotFound);
        }
        let prev = USER.with_borrow_mut(|m| m.insert(user.id, user.clone()));
        self.save_indexes(&user, prev.as_ref());
        Ok(user)
    }

    fn delete(&self, id: &UserId) -> RepositoryResult<UserId> {
        let old = USER.with_borrow_mut(|m| m.remove(id));
        if old.is_none() {
            Err(RepositoryError::NotFound)
        } else {
            self.remove_indexes(&old.unwrap());
            Ok(*id)
        }
    }
}

impl IdentityProvider for UserRepository {
    fn get_user(&self, identity: Principal) -> Option<User> {
        self.identity_index
            .find(identity, None, 0)
            .iter()
            .filter_map(|id| self.get(id))
            .last()
    }
}

lazy_static! {
    pub static ref MESSAGE_REPOSITORY: Arc<MessageRepository> =
        Arc::new(MessageRepository::default());
    pub static ref CONVERSATION_REPOSITORY: Arc<ConversationRepository> =
        Arc::new(ConversationRepository::default());
    pub static ref USER_REPOSITORY: Arc<UserRepository> = Arc::new(UserRepository::default());
}

#[cfg(test)]
mod tests {
    use crate::utils::mock_ic0;

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

    fn reset_user_data() {
        USER.with_borrow_mut(|m| m.clear_new());
        USER_PRINCIPAL_INDEX.with_borrow_mut(|m| m.clear_new());
        NEXT_USER_ID.with_borrow_mut(|m| m.set(1).unwrap());
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
    fn parse_roles_from_str_should_works() {
        let r = "user".parse::<Roles>().unwrap();
        assert!(matches!(r, Roles::User));
        let r: Roles = "assistant".parse().unwrap();
        assert!(matches!(r, Roles::Assistant));
        let s = String::from("system");
        let r: Roles = s.parse().unwrap();
        assert!(matches!(r, Roles::System));
    }

    #[test]
    #[should_panic]
    fn parse_roles_from_unknown_str_should_failed() {
        "jedi".parse::<Roles>().unwrap();
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
            fullname: "test_user".to_string(),
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
    fn delete_message_should_work() {
        reset_msg_data();
        let repo = MessageRepository::default();
        (0..3).for_each(|i| {
            repo.insert(Message {
                id: 0,
                conversation: 0,
                content: format!("number-{}", i),
                timestamp: 0,
                role: Roles::User,
            })
            .unwrap();
        });
        repo.delete(&2).unwrap();
        assert!(repo.get(&2).is_none());
    }

    #[test]
    #[should_panic]
    fn delete_non_exist_message_should_failed() {
        reset_msg_data();
        let repo = MessageRepository::default();
        (0..3).for_each(|i| {
            repo.insert(Message {
                id: 0,
                conversation: 0,
                content: format!("number-{}", i),
                timestamp: 0,
                role: Roles::User,
            })
            .unwrap();
        });
        repo.delete(&10).unwrap();
    }

    #[test]
    fn delete_messge_by_conversation_should_work() {
        reset_msg_data();
        let repo = MessageRepository::default();
        (0..3).for_each(|i| {
            repo.insert(Message {
                id: 0,
                conversation: 1,
                content: format!("number-{}", i),
                timestamp: 0,
                role: Roles::User,
            })
            .unwrap();
        });
        (0..5).for_each(|i| {
            repo.insert(Message {
                id: 0,
                conversation: 7,
                content: format!("number-{}", i),
                timestamp: 0,
                role: Roles::User,
            })
            .unwrap();
        });
        repo.delete_by_conversation(&1).unwrap();
        assert!(repo.paged_list(1, None, usize::default()).1.is_empty());
        assert_eq!(5, repo.paged_list(7, None, usize::default()).1.len());
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
        let mut conversation = Conversation {
            id: 1,
            user: 1,
            updated_at: 1234567890,
            name: "Test Conversation".to_string(),
        };
        repo.upsert(conversation.clone()).unwrap();
        assert!(repo.get(&1).is_some());
        assert_eq!("Test Conversation", repo.get(&1).unwrap().name);
        conversation.name = "Updated Conversation".to_string();
        repo.upsert(conversation).unwrap();
        assert_eq!("Updated Conversation", repo.get(&1).unwrap().name);
    }

    #[test]
    fn delete_conversation_should_work() {
        reset_conv_data();
        let repo = ConversationRepository::default();
        repo.insert(Conversation {
            id: 0,
            user: 1,
            updated_at: 0,
            name: String::from("abc"),
        })
        .unwrap();
        assert!(repo.get(&1).is_some());
        repo.delete(&1).unwrap();
        assert!(repo.get(&1).is_none());
    }

    #[test]
    #[should_panic]
    fn delete_non_exist_conversation_should_failed() {
        reset_conv_data();
        let repo = ConversationRepository::default();
        repo.insert(Conversation {
            id: 0,
            user: 1,
            updated_at: 0,
            name: String::from("abc"),
        })
        .unwrap();
        assert!(repo.get(&1).is_some());
        repo.delete(&3).unwrap();
    }

    #[test]
    fn conversation_cursor_paged_list_should_return_correct_list() {
        reset_conv_data();
        mock_ic0::reset_timestamp_to(1);
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

    #[test]
    fn get_and_insert_user_should_work() {
        reset_user_data();
        let repo = UserRepository::default();
        let user = User {
            id: 0,
            fullname: "fulan".to_string(),
            identity: Principal::anonymous(),
            resume: "profile".to_string(),
        };
        repo.insert(user).unwrap();
        assert!(repo.get(&1).is_some());
        assert_eq!("fulan", repo.get(&1).unwrap().fullname);
    }

    #[test]
    fn update_user_should_work() {
        reset_user_data();
        let repo = UserRepository::default();
        repo.insert(User {
            id: 0,
            fullname: "fulan".to_string(),
            identity: Principal::anonymous(),
            resume: "profile".to_string(),
        })
        .unwrap();
        assert!(repo.get(&1).is_some());
        assert_eq!("fulan", repo.get(&1).unwrap().fullname);
        repo.update(User {
            id: 1,
            fullname: "fulanah".to_string(),
            identity: Principal::anonymous(),
            resume: "profile".to_string(),
        })
        .unwrap();
        assert_eq!("fulanah", repo.get(&1).unwrap().fullname);
    }

    #[test]
    fn delete_user_should_work() {
        reset_user_data();
        let repo = UserRepository::default();
        repo.insert(User {
            id: 0,
            fullname: "fulan".to_string(),
            identity: Principal::anonymous(),
            resume: "profile".to_string(),
        })
        .unwrap();
        assert!(repo.get(&1).is_some());
        repo.delete(&1).unwrap();
        assert!(repo.get(&1).is_none());
    }

    #[test]
    #[should_panic]
    fn delete_non_exist_user_should_failed() {
        reset_user_data();
        let repo = UserRepository::default();
        repo.insert(User {
            id: 0,
            fullname: "fulan".to_string(),
            identity: Principal::anonymous(),
            resume: "profile".to_string(),
        })
        .unwrap();
        assert!(repo.get(&1).is_some());
        assert!(repo.get(&2).is_none());
        repo.delete(&2).unwrap();
    }

    #[test]
    fn get_user_by_identity_should_work() {
        reset_user_data();
        let repo = UserRepository::default();

        let identity = Principal::from_text("2chl6-4hpzw-vqaaa-aaaaa-c").unwrap();
        repo.insert(User {
            id: 0,
            fullname: "user1".to_string(),
            identity: identity.clone(),
            resume: "user1".to_string(),
        })
        .unwrap();
        repo.insert(User {
            id: 0,
            fullname: "user2".to_string(),
            identity: Principal::anonymous(),
            resume: "user2".to_string(),
        })
        .unwrap();
        let q = repo.get_user(identity);
        assert!(q.is_some());
        assert_eq!("user1", q.unwrap().fullname);
    }
}

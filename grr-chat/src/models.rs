//! Google Chat API response models deserialized with serde

use serde::{Deserialize, Serialize};

/// A space (chat room or direct message) the user belongs to
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Space {
    pub name: Option<String>,
    pub display_name: Option<String>,
    /// The JSON field is the keyword "type"; it cannot be a Rust field.
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub space_type: Option<String>,
    pub member_count: Option<i32>,
    pub last_active_time: Option<String>,
    pub threading_state: Option<String>,
}

/// The user or app that sent a message
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sender {
    pub name: Option<String>,
    pub display_name: Option<String>,
    /// The JSON field is the keyword "type"; it cannot be a Rust field.
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub is_anonymous: Option<bool>,
}

/// The thread a message belongs to
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Thread {
    pub name: Option<String>,
    pub thread_key: Option<String>,
}

/// A message in a space
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub name: Option<String>,
    pub sender: Option<Sender>,
    pub create_time: Option<String>,
    pub text: Option<String>,
    pub thread: Option<Thread>,
    pub deleted: Option<bool>,
    pub formatted_time: Option<String>,
    pub argument_text: Option<String>,
}

/// Envelope of spaces.list (one result page)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListSpacesResponse {
    pub spaces: Option<Vec<Space>>,
    pub next_page_token: Option<String>,
}

/// Envelope of spaces.messages.list (one result page)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListMessagesResponse {
    pub messages: Option<Vec<Message>>,
    pub next_page_token: Option<String>,
}

/// Body of spaces.messages.create
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMessageRequest {
    pub text: String,
}

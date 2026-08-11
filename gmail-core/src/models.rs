//! Gmail API response models with zero-copy support via rkyv (disabled for now due to recursive type issues)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Gmail message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: String,
    pub thread_id: String,
    pub label_ids: Vec<String>,
    pub snippet: Option<String>,
    pub history_id: Option<String>,
    pub internal_date: Option<String>,
    pub payload: Option<MessagePayload>,
    pub size_estimate: Option<u64>,
    pub raw: Option<String>,
}

/// Message payload with headers and body
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessagePayload {
    pub part_id: Option<String>,
    pub mime_type: String,
    pub filename: Option<String>,
    pub headers: Vec<Header>,
    pub body: MessageBody,
    pub parts: Option<Vec<MessagePayload>>,
}

/// Message body
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageBody {
    pub attachment_id: Option<String>,
    pub size: u64,
    pub data: Option<String>,
}

/// Email header
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Header {
    pub name: String,
    pub value: String,
}

/// Thread
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Thread {
    pub id: String,
    pub snippet: Option<String>,
    pub history_id: Option<String>,
    pub messages: Option<Vec<Message>>,
}

/// Label
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Label {
    pub id: String,
    pub name: String,
    pub message_list_visibility: Option<String>,
    pub label_list_visibility: Option<String>,
    pub r#type: Option<String>,
    pub messages_total: Option<u64>,
    pub messages_unread: Option<u64>,
    pub threads_total: Option<u64>,
    pub threads_unread: Option<u64>,
    pub color: Option<LabelColor>,
}

/// Label color
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LabelColor {
    pub text_color: String,
    pub background_color: String,
}

/// Draft
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Draft {
    pub id: String,
    pub message: Message,
}

/// Attachment
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    pub size: u64,
    pub data: String,
}

/// Send-as alias
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendAs {
    pub send_as_email: String,
    pub display_name: Option<String>,
    pub reply_to_address: Option<String>,
    pub signature: Option<String>,
    pub is_primary: bool,
    pub is_default: bool,
    pub treat_as_alias: bool,
    pub verification_status: Option<String>,
}

/// History record
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct History {
    pub id: String,
    pub messages: Option<Vec<Message>>,
    pub messages_added: Option<Vec<HistoryMessageAdded>>,
    pub messages_deleted: Option<Vec<HistoryMessageDeleted>>,
    pub labels_added: Option<Vec<HistoryLabelAdded>>,
    pub labels_removed: Option<Vec<HistoryLabelRemoved>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryMessageAdded {
    pub message: Message,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryMessageDeleted {
    pub message: Message,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryLabelAdded {
    pub message: Message,
    pub label_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryLabelRemoved {
    pub message: Message,
    pub label_ids: Vec<String>,
}

/// User profile
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub email_address: String,
    pub messages_total: u64,
    pub threads_total: u64,
    pub history_id: String,
}

/// Watch response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchResponse {
    pub history_id: String,
    pub expiration: String,
}

/// Search response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
    pub messages: Vec<MessageRef>,
    pub next_page_token: Option<String>,
    pub result_size_estimate: Option<u64>,
}

/// Message reference (lightweight)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageRef {
    pub id: String,
    pub thread_id: String,
}

/// List response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListResponse<T> {
    pub items: Vec<T>,
    pub next_page_token: Option<String>,
    pub result_size_estimate: Option<u64>,
}

/// Batch request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchRequest {
    pub requests: Vec<BatchRequestEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchRequestEntry {
    pub id: String,
    pub method: String,
    pub path: String,
    pub body: Option<serde_json::Value>,
    pub headers: Option<HashMap<String, String>>,
}

/// Batch response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchResponse {
    pub responses: Vec<BatchResponseEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchResponseEntry {
    pub id: String,
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Option<serde_json::Value>,
}

/// Modify labels request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModifyLabelsRequest {
    pub add_label_ids: Vec<String>,
    pub remove_label_ids: Vec<String>,
}

/// Batch modify labels request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchModifyLabelsRequest {
    pub ids: Vec<String>,
    pub add_label_ids: Vec<String>,
    pub remove_label_ids: Vec<String>,
}

/// Create label request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateLabelRequest {
    pub name: String,
    pub label_list_visibility: Option<String>,
    pub message_list_visibility: Option<String>,
    pub color: Option<LabelColor>,
}

/// Update label request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateLabelRequest {
    pub name: Option<String>,
    pub label_list_visibility: Option<String>,
    pub message_list_visibility: Option<String>,
    pub color: Option<LabelColor>,
}

/// Watch request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchRequest {
    pub topic_name: String,
    pub label_ids: Option<Vec<String>>,
    pub label_filter_action: Option<String>,
}

/// Import message request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportMessageRequest {
    pub raw: String,
    pub internal_date_source: Option<String>,
    pub deleted: Option<bool>,
    pub never_spam: Option<bool>,
    pub process_for_calendar: Option<bool>,
}

/// Create draft request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDraftRequest {
    pub message: DraftMessage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftMessage {
    pub raw: String,
    pub thread_id: Option<String>,
}

/// Send message request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageRequest {
    pub raw: String,
    pub thread_id: Option<String>,
}

/// Attachment data for sending emails
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentData {
    pub filename: String,
    pub content: Vec<u8>,
    pub mime_type: String,
}

/// Options for creating a label
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateLabelOptions {
    pub label_list_visibility: Option<String>,
    pub message_list_visibility: Option<String>,
    pub color: Option<LabelColor>,
}

/// Options for updating a label
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateLabelOptions {
    pub name: Option<String>,
    pub label_list_visibility: Option<String>,
    pub message_list_visibility: Option<String>,
    pub color: Option<LabelColor>,
}

/// Options for creating a send-as alias
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSendAsOptions {
    pub send_as_email: String,
    pub display_name: Option<String>,
    pub reply_to_address: Option<String>,
    pub signature: Option<String>,
    pub treat_as_alias: Option<bool>,
}

/// Options for updating a send-as alias
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSendAsOptions {
    pub display_name: Option<String>,
    pub reply_to_address: Option<String>,
    pub signature: Option<String>,
    pub treat_as_alias: Option<bool>,
}

/// Error response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    pub error: ErrorDetail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorDetail {
    pub code: u16,
    pub message: String,
    pub status: String,
    pub details: Option<Vec<serde_json::Value>>,
}

// Lightweight message for search results
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmailMessage {
    pub id: String,
    pub thread_id: String,
    pub subject: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub date: Option<String>,
    pub snippet: Option<String>,
    pub labels: Option<Vec<String>>,
    pub size_estimate: Option<u64>,
    pub internal_date: Option<String>,
}

// Full message with body and attachments
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadMessage {
    pub id: String,
    pub thread_id: String,
    pub subject: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub date: Option<String>,
    pub body: Option<String>,
    pub labels: Option<Vec<String>>,
    pub size_estimate: Option<u64>,
    pub internal_date: Option<String>,
    pub attachments: Option<Vec<AttachmentInfo>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentInfo {
    pub attachment_id: String,
    pub filename: String,
    pub mime_type: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftInfo {
    pub id: String,
    pub message: EmailMessage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LabelInfo {
    pub id: String,
    pub name: String,
    pub message_list_visibility: Option<String>,
    pub label_list_visibility: Option<String>,
    pub r#type: Option<String>,
    pub messages_total: Option<u64>,
    pub messages_unread: Option<u64>,
    pub threads_total: Option<u64>,
    pub threads_unread: Option<u64>,
    pub color: Option<LabelColor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryRecord {
    pub id: String,
    pub messages: Option<Vec<EmailMessage>>,
    pub labels_added: Option<Vec<String>>,
    pub labels_removed: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendAsInfo {
    pub send_as_email: String,
    pub display_name: Option<String>,
    pub reply_to_address: Option<String>,
    pub signature: Option<String>,
    pub is_primary: Option<bool>,
    pub is_default: Option<bool>,
    pub treat_as_alias: Option<bool>,
}

/// Helper to extract header value
impl MessagePayload {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|h| h.name.eq_ignore_ascii_case(name))
            .map(|h| h.value.as_str())
    }

    pub fn subject(&self) -> Option<&str> {
        self.header("Subject")
    }

    pub fn from(&self) -> Option<&str> {
        self.header("From")
    }

    pub fn to(&self) -> Option<&str> {
        self.header("To")
    }

    pub fn date(&self) -> Option<&str> {
        self.header("Date")
    }

    pub fn message_id(&self) -> Option<&str> {
        self.header("Message-ID")
    }

    pub fn in_reply_to(&self) -> Option<&str> {
        self.header("In-Reply-To")
    }

    pub fn references(&self) -> Option<&str> {
        self.header("References")
    }
}
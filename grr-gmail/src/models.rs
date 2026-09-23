//! Gmail API response models deserialized with serde

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Gmail message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: String,
    pub thread_id: String,
    // Gmail omits labelIds on minimal-format messages (e.g. history entries).
    #[serde(default)]
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
    // Gmail omits false booleans instead of sending `false`.
    #[serde(default)]
    pub is_primary: bool,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default)]
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
    #[serde(default)]
    pub label_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryLabelRemoved {
    pub message: Message,
    #[serde(default)]
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
    // Gmail omits the key instead of sending `[]` when there are no hits.
    #[serde(default)]
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

/// List response wrappers.
///
/// Gmail keys list payloads by resource and omits the key instead of
/// sending an empty array, so each resource gets its own plain serde
/// struct with a defaulted vector — no aliases, no custom impls.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LabelList {
    #[serde(default)]
    pub labels: Vec<Label>,
    pub next_page_token: Option<String>,
    pub result_size_estimate: Option<u64>,
}

/// List response wrapper.
///
/// Gmail keys list payloads by resource and omits the key instead of
/// sending an empty array, so each resource gets its own plain serde
/// struct with a defaulted vector — no aliases, no custom impls.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftList {
    #[serde(default)]
    pub drafts: Vec<Draft>,
    pub next_page_token: Option<String>,
    pub result_size_estimate: Option<u64>,
}

/// List response wrapper.
///
/// Gmail keys list payloads by resource and omits the key instead of
/// sending an empty array, so each resource gets its own plain serde
/// struct with a defaulted vector — no aliases, no custom impls.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryList {
    #[serde(default)]
    pub history: Vec<History>,
    pub next_page_token: Option<String>,
}

/// List response wrapper.
///
/// Gmail keys list payloads by resource and omits the key instead of
/// sending an empty array, so each resource gets its own plain serde
/// struct with a defaulted vector — no aliases, no custom impls.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendAsList {
    #[serde(default)]
    pub send_as: Vec<SendAs>,
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

/// Extract the readable message body: first `text/plain` part found in a
/// DFS walk, falling back to `text/html` (returned as-is; agents strip
/// tags downstream). Attachment bodies are skipped.
pub fn extract_body(msg: &Message) -> Option<String> {
    let payload = msg.payload.as_ref()?;
    let plain = find_part_body(payload, "text/plain");
    if let Some(text) = plain {
        return Some(text);
    }
    find_part_body(payload, "text/html")
}

fn find_part_body(part: &MessagePayload, mime: &str) -> Option<String> {
    if part.mime_type == mime
        && let Some(encoded) = part.body.data.as_deref().filter(|d| !d.is_empty())
    {
        // Ignore decode failures: a corrupted part must not hide
        // readable parts deeper in the tree.
        if let Some(decoded) = decode_body_part(encoded) {
            return Some(decoded);
        }
    }
    let parts = part.parts.as_ref()?;
    for child in parts {
        if let Some(text) = find_part_body(child, mime) {
            return Some(text);
        }
    }
    None
}

fn decode_body_part(encoded: &str) -> Option<String> {
    use base64::Engine;
    // Gmail documents bodies as unpadded base64url, but parts also arrive
    // in the standard alphabet (with `/` and `+`). Normalize the alphabet
    // and re-pad before one canonical decode.
    let mut buf = encoded.replace('-', "+").replace('_', "/");
    while !buf.len().is_multiple_of(4) {
        buf.push('=');
    }
    let bytes = base64::engine::general_purpose::STANDARD.decode(buf).ok()?;
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

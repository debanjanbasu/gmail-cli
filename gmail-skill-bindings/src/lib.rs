//! gmail-skill-bindings - NAPI bindings for opencode skill integration.
#![allow(clippy::trailing_empty_array)]

use gmail_core::{
    AttachmentData, CreateLabelOptions, CreateSendAsOptions, LabelColor,
    UpdateLabelOptions, UpdateSendAsOptions,
};
use napi_derive::napi;

/// JSON serialization helper for API responses.
///
/// Serializes a `serde::Serialize` value into a JSON string, falling back to
/// `{}` if serialization fails.
fn to_json_str<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string())
}

/// Colors used to render a label.
#[napi(object)]
pub struct NapiLabelColor {
    /// Text color as a hex string.
    pub text_color: String,
    /// Background color as a hex string.
    pub background_color: String,
}

/// Options for creating a label.
#[napi(object)]
pub struct NapiCreateLabelOptions {
    /// How the label is shown in the message list.
    pub label_list_visibility: Option<String>,
    /// How the label is shown in the message list pane.
    pub message_list_visibility: Option<String>,
    /// Optional label color.
    pub color: Option<NapiLabelColor>,
}

/// Options for updating a label.
#[napi(object)]
pub struct NapiUpdateLabelOptions {
    /// New label name.
    pub name: Option<String>,
    /// How the label is shown in the message list.
    pub label_list_visibility: Option<String>,
    /// How the label is shown in the message list pane.
    pub message_list_visibility: Option<String>,
    /// Optional label color.
    pub color: Option<NapiLabelColor>,
}

/// Options for creating a send-as alias.
#[napi(object)]
pub struct NapiCreateSendAsOptions {
    /// The email address that appears in the "From" header.
    pub send_as_email: String,
    /// Optional display name.
    pub display_name: Option<String>,
    /// Optional reply-to address.
    pub reply_to_address: Option<String>,
    /// Optional signature appended to outgoing mail.
    pub signature: Option<String>,
    /// Whether this address should appear as an alias.
    pub treat_as_alias: Option<bool>,
}

/// Options for updating a send-as alias.
#[napi(object)]
pub struct NapiUpdateSendAsOptions {
    /// Optional display name.
    pub display_name: Option<String>,
    /// Optional reply-to address.
    pub reply_to_address: Option<String>,
    /// Optional signature appended to outgoing mail.
    pub signature: Option<String>,
    /// Whether this address should appear as an alias.
    pub treat_as_alias: Option<bool>,
}

/// A file attachment to send with a message.
#[napi(object)]
pub struct NapiAttachmentData {
    /// The attachment file name.
    pub filename: String,
    /// The raw attachment bytes.
    pub content: Vec<u8>,
    /// The MIME type of the attachment.
    pub mime_type: String,
}

impl From<NapiAttachmentData> for AttachmentData {
    fn from(attachment: NapiAttachmentData) -> Self {
        Self {
            filename: attachment.filename,
            content: attachment.content,
            mime_type: attachment.mime_type,
        }
    }
}

impl From<NapiCreateLabelOptions> for CreateLabelOptions {
    fn from(options: NapiCreateLabelOptions) -> Self {
        Self {
            label_list_visibility: options.label_list_visibility,
            message_list_visibility: options.message_list_visibility,
            color: options.color.map(|color| LabelColor {
                text_color: color.text_color,
                background_color: color.background_color,
            }),
        }
    }
}

impl From<NapiUpdateLabelOptions> for UpdateLabelOptions {
    fn from(options: NapiUpdateLabelOptions) -> Self {
        Self {
            name: options.name,
            label_list_visibility: options.label_list_visibility,
            message_list_visibility: options.message_list_visibility,
            color: options.color.map(|color| LabelColor {
                text_color: color.text_color,
                background_color: color.background_color,
            }),
        }
    }
}

impl From<NapiCreateSendAsOptions> for CreateSendAsOptions {
    fn from(options: NapiCreateSendAsOptions) -> Self {
        Self {
            send_as_email: options.send_as_email,
            display_name: options.display_name,
            reply_to_address: options.reply_to_address,
            signature: options.signature,
            treat_as_alias: options.treat_as_alias,
        }
    }
}

impl From<NapiUpdateSendAsOptions> for UpdateSendAsOptions {
    fn from(options: NapiUpdateSendAsOptions) -> Self {
        Self {
            display_name: options.display_name,
            reply_to_address: options.reply_to_address,
            signature: options.signature,
            treat_as_alias: options.treat_as_alias,
        }
    }
}

/// Wraps a [`gmail_core::GmailClient`] for JavaScript interop.
///
/// All methods serialize their results to JSON strings for safe interop with
/// the opencode skill.
#[napi]
pub struct Gmail {
    client: gmail_core::GmailClient,
}

#[napi]
impl Gmail {
    /// Build a new [`Gmail`] client from the config at
    /// `~/.gmail-opencode/config.toml`.
    ///
    /// # Errors
    ///
    /// Returns an error if configuration loading, authentication, or client
    /// construction fails.
    #[napi(factory)]
    pub async fn create() -> napi::Result<Self> {
        let config = gmail_core::ConfigLoader::load()
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        let auth = gmail_core::GmailAuth::new(config.oauth.clone())
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        let client = gmail_core::GmailClientBuilder::new(config)
            .auth(auth)
            .build()
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(Self { client })
    }

    /// Search messages matching a query.
    ///
    /// # Errors
    ///
    /// Returns an error if the Gmail API request fails.
    #[napi]
    pub async fn search(&self, query: String, max_results: u32) -> napi::Result<String> {
        let results = self
            .client
            .search(&query, max_results as usize)
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(to_json_str(&results))
    }

    /// Get a thread by id.
    ///
    /// # Errors
    ///
    /// Returns an error if the Gmail API request fails.
    #[napi]
    pub async fn get_thread(&self, thread_id: String) -> napi::Result<String> {
        let thread = self
            .client
            .get_thread(&thread_id)
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(to_json_str(&thread))
    }

    /// Get a message by id, optionally requesting a specific format.
    ///
    /// # Errors
    ///
    /// Returns an error if the Gmail API request fails.
    #[napi]
    pub async fn get_message(
        &self,
        message_id: String,
        format: Option<String>,
    ) -> napi::Result<String> {
        let message = self
            .client
            .get_message(&message_id, format.as_deref())
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(to_json_str(&message))
    }

    /// Get only the metadata for a message.
    ///
    /// # Errors
    ///
    /// Returns an error if the Gmail API request fails.
    #[napi]
    pub async fn get_message_metadata(&self, message_id: String) -> napi::Result<String> {
        let message = self
            .client
            .get_message_metadata(&message_id)
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(to_json_str(&message))
    }

    /// Get the raw RFC 822 source of a message.
    ///
    /// # Errors
    ///
    /// Returns an error if the Gmail API request fails.
    #[napi]
    pub async fn get_message_raw(&self, message_id: String) -> napi::Result<String> {
        let raw = self
            .client
            .get_message_raw(&message_id)
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(raw)
    }

    /// Get an attachment belonging to a message.
    ///
    /// # Errors
    ///
    /// Returns an error if the Gmail API request fails.
    #[napi]
    pub async fn get_attachment(
        &self,
        message_id: String,
        attachment_id: String,
    ) -> napi::Result<String> {
        let attachment = self
            .client
            .get_attachment(&message_id, &attachment_id)
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(to_json_str(&attachment))
    }

    /// Send a plain text message.
    ///
    /// # Errors
    ///
    /// Returns an error if the Gmail API request fails.
    #[napi]
    pub async fn send_message(
        &self,
        to: String,
        subject: String,
        body: String,
    ) -> napi::Result<String> {
        let message = self
            .client
            .send(&to, &subject, &body)
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(to_json_str(&message))
    }

    /// Send a message with optional cc and bcc recipients.
    ///
    /// # Errors
    ///
    /// Returns an error if the Gmail API request fails.
    #[napi]
    pub async fn send_with_options(
        &self,
        to: String,
        subject: String,
        body: String,
        cc: Option<String>,
        bcc: Option<String>,
    ) -> napi::Result<String> {
        let message = self
            .client
            .send_with_options(&to, &subject, &body, cc.as_deref(), bcc.as_deref())
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(to_json_str(&message))
    }

    /// Send a message with file attachments.
    ///
    /// # Errors
    ///
    /// Returns an error if the Gmail API request fails.
    #[napi]
    pub async fn send_with_attachments(
        &self,
        to: String,
        subject: String,
        body: String,
        attachments: Vec<NapiAttachmentData>,
        thread_id: Option<String>,
    ) -> napi::Result<String> {
        let attachments: Vec<AttachmentData> = attachments.into_iter().map(Into::into).collect();
        let message = self
            .client
            .send_with_attachments(&to, &subject, &body, attachments, thread_id.as_deref())
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(to_json_str(&message))
    }

    /// Create a draft message.
    ///
    /// # Errors
    ///
    /// Returns an error if the Gmail API request fails.
    #[napi]
    pub async fn create_draft(
        &self,
        to: String,
        subject: String,
        body: String,
    ) -> napi::Result<String> {
        let draft = self
            .client
            .create_draft(&to, &subject, &body)
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(to_json_str(&draft))
    }

    /// List drafts, up to `max_results`.
    ///
    /// # Errors
    ///
    /// Returns an error if the Gmail API request fails.
    #[napi]
    pub async fn list_drafts(&self, max_results: u32) -> napi::Result<String> {
        let drafts = self
            .client
            .list_drafts(max_results as usize)
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(to_json_str(&drafts))
    }

    /// Get a draft by id.
    ///
    /// # Errors
    ///
    /// Returns an error if the Gmail API request fails.
    #[napi]
    pub async fn get_draft(&self, draft_id: String) -> napi::Result<String> {
        let draft = self
            .client
            .get_draft(&draft_id)
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(to_json_str(&draft))
    }

    /// Update the content of a draft.
    ///
    /// # Errors
    ///
    /// Returns an error if the Gmail API request fails.
    #[napi]
    pub async fn update_draft(
        &self,
        draft_id: String,
        to: String,
        subject: String,
        body: String,
    ) -> napi::Result<String> {
        let draft = self
            .client
            .update_draft(&draft_id, &to, &subject, &body)
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(to_json_str(&draft))
    }

    /// Delete a draft.
    ///
    /// # Errors
    ///
    /// Returns an error if the Gmail API request fails.
    #[napi]
    pub async fn delete_draft(&self, draft_id: String) -> napi::Result<()> {
        self.client
            .delete_draft(&draft_id)
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))
    }

    /// Send an existing draft.
    ///
    /// # Errors
    ///
    /// Returns an error if the Gmail API request fails.
    #[napi]
    pub async fn send_draft(&self, draft_id: String) -> napi::Result<String> {
        let message = self
            .client
            .send_draft(&draft_id)
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(to_json_str(&message))
    }

    /// List all labels.
    ///
    /// # Errors
    ///
    /// Returns an error if the Gmail API request fails.
    #[napi]
    pub async fn list_labels(&self) -> napi::Result<String> {
        let labels = self
            .client
            .list_labels()
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(to_json_str(&labels))
    }

    /// Get a label by id.
    ///
    /// # Errors
    ///
    /// Returns an error if the Gmail API request fails.
    #[napi]
    pub async fn get_label(&self, label_id: String) -> napi::Result<String> {
        let label = self
            .client
            .get_label(&label_id)
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(to_json_str(&label))
    }

    /// Create a label.
    ///
    /// # Errors
    ///
    /// Returns an error if the Gmail API request fails.
    #[napi]
    pub async fn create_label(
        &self,
        name: String,
        options: NapiCreateLabelOptions,
    ) -> napi::Result<String> {
        let options: CreateLabelOptions = options.into();
        let label = self
            .client
            .create_label(&name, options)
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(to_json_str(&label))
    }

    /// Update a label.
    ///
    /// # Errors
    ///
    /// Returns an error if the Gmail API request fails.
    #[napi]
    pub async fn update_label(
        &self,
        label_id: String,
        options: NapiUpdateLabelOptions,
    ) -> napi::Result<String> {
        let options: UpdateLabelOptions = options.into();
        let label = self
            .client
            .update_label(&label_id, options)
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(to_json_str(&label))
    }

    /// Delete a label.
    ///
    /// # Errors
    ///
    /// Returns an error if the Gmail API request fails.
    #[napi]
    pub async fn delete_label(&self, label_id: String) -> napi::Result<()> {
        self.client
            .delete_label(&label_id)
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))
    }

    /// Add and/or remove labels on a message.
    ///
    /// # Errors
    ///
    /// Returns an error if the Gmail API request fails.
    #[napi]
    pub async fn modify_message_labels(
        &self,
        message_id: String,
        add_labels: Vec<String>,
        remove_labels: Vec<String>,
    ) -> napi::Result<String> {
        let message = self
            .client
            .modify_labels(&message_id, &add_labels, &remove_labels)
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(to_json_str(&message))
    }

    /// Batch add and/or remove labels across multiple messages.
    ///
    /// # Errors
    ///
    /// Returns an error if the Gmail API request fails.
    #[napi]
    pub async fn batch_modify_messages(
        &self,
        message_ids: Vec<String>,
        add_labels: Vec<String>,
        remove_labels: Vec<String>,
    ) -> napi::Result<()> {
        self.client
            .batch_modify_labels(&message_ids, &add_labels, &remove_labels)
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))
    }

    /// Batch delete messages.
    ///
    /// # Errors
    ///
    /// Returns an error if the Gmail API request fails.
    #[napi]
    pub async fn batch_delete_messages(&self, message_ids: Vec<String>) -> napi::Result<()> {
        self.client
            .batch_delete_messages(&message_ids)
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))
    }

    /// Move a message to the trash.
    ///
    /// # Errors
    ///
    /// Returns an error if the Gmail API request fails.
    #[napi]
    pub async fn trash_message(&self, message_id: String) -> napi::Result<String> {
        let message = self
            .client
            .trash_message(&message_id)
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(to_json_str(&message))
    }

    /// Remove a message from the trash.
    ///
    /// # Errors
    ///
    /// Returns an error if the Gmail API request fails.
    #[napi]
    pub async fn untrash_message(&self, message_id: String) -> napi::Result<String> {
        let message = self
            .client
            .untrash_message(&message_id)
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(to_json_str(&message))
    }

    /// Permanently delete a message.
    ///
    /// # Errors
    ///
    /// Returns an error if the Gmail API request fails.
    #[napi]
    pub async fn delete_message(&self, message_id: String) -> napi::Result<()> {
        self.client
            .delete_message(&message_id)
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))
    }

    /// Add and/or remove labels on every message in a thread.
    ///
    /// # Errors
    ///
    /// Returns an error if the Gmail API request fails.
    #[napi]
    pub async fn modify_thread_labels(
        &self,
        thread_id: String,
        add_labels: Vec<String>,
        remove_labels: Vec<String>,
    ) -> napi::Result<()> {
        self.client
            .modify_thread_labels(&thread_id, &add_labels, &remove_labels)
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))
    }

    /// Move a thread to the trash.
    ///
    /// # Errors
    ///
    /// Returns an error if the Gmail API request fails.
    #[napi]
    pub async fn trash_thread(&self, thread_id: String) -> napi::Result<()> {
        self.client
            .trash_thread(&thread_id)
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))
    }

    /// Remove a thread from the trash.
    ///
    /// # Errors
    ///
    /// Returns an error if the Gmail API request fails.
    #[napi]
    pub async fn untrash_thread(&self, thread_id: String) -> napi::Result<()> {
        self.client
            .untrash_thread(&thread_id)
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))
    }

    /// Permanently delete a thread.
    ///
    /// # Errors
    ///
    /// Returns an error if the Gmail API request fails.
    #[napi]
    pub async fn delete_thread(&self, thread_id: String) -> napi::Result<()> {
        self.client
            .delete_thread(&thread_id)
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))
    }

    /// Get the change history for the mailbox.
    ///
    /// # Errors
    ///
    /// Returns an error if the Gmail API request fails.
    #[napi]
    pub async fn get_history(
        &self,
        start_history_id: String,
        label_id: Option<String>,
        max_results: u32,
    ) -> napi::Result<String> {
        let history = self
            .client
            .get_history(&start_history_id, label_id.as_deref(), max_results as usize)
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(to_json_str(&history))
    }

    /// Get the user's profile.
    ///
    /// # Errors
    ///
    /// Returns an error if the Gmail API request fails.
    #[napi]
    pub async fn get_profile(&self) -> napi::Result<String> {
        let profile = self
            .client
            .get_profile()
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(to_json_str(&profile))
    }

    /// List all send-as aliases.
    ///
    /// # Errors
    ///
    /// Returns an error if the Gmail API request fails.
    #[napi]
    pub async fn list_send_as(&self) -> napi::Result<String> {
        let send_as = self
            .client
            .list_send_as()
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(to_json_str(&send_as))
    }

    /// Get a send-as alias.
    ///
    /// # Errors
    ///
    /// Returns an error if the Gmail API request fails.
    #[napi]
    pub async fn get_send_as(&self, send_as_email: String) -> napi::Result<String> {
        let send_as = self
            .client
            .get_send_as(&send_as_email)
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(to_json_str(&send_as))
    }

    /// Create a send-as alias.
    ///
    /// # Errors
    ///
    /// Returns an error if the Gmail API request fails.
    #[napi]
    pub async fn create_send_as(
        &self,
        options: NapiCreateSendAsOptions,
    ) -> napi::Result<String> {
        let options: CreateSendAsOptions = options.into();
        let send_as = self
            .client
            .create_send_as(options)
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(to_json_str(&send_as))
    }

    /// Update a send-as alias.
    ///
    /// # Errors
    ///
    /// Returns an error if the Gmail API request fails.
    #[napi]
    pub async fn update_send_as(
        &self,
        send_as_email: String,
        options: NapiUpdateSendAsOptions,
    ) -> napi::Result<String> {
        let options: UpdateSendAsOptions = options.into();
        let send_as = self
            .client
            .update_send_as(&send_as_email, options)
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(to_json_str(&send_as))
    }

    /// Delete a send-as alias.
    ///
    /// # Errors
    ///
    /// Returns an error if the Gmail API request fails.
    #[napi]
    pub async fn delete_send_as(&self, send_as_email: String) -> napi::Result<()> {
        self.client
            .delete_send_as(&send_as_email)
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))
    }

    /// Enable push notifications for the mailbox.
    ///
    /// # Errors
    ///
    /// Returns an error if the Gmail API request fails.
    #[napi]
    pub async fn watch(
        &self,
        topic_name: String,
        label_ids: Option<Vec<String>>,
    ) -> napi::Result<String> {
        let response = self
            .client
            .watch(&topic_name, label_ids)
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(to_json_str(&response))
    }

    /// Disable push notifications for the mailbox.
    ///
    /// # Errors
    ///
    /// Returns an error if the Gmail API request fails.
    #[napi]
    pub async fn stop_watch(&self) -> napi::Result<()> {
        self.client
            .stop_watch()
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))
    }

    /// Import a message from raw RFC 822 source.
    ///
    /// # Errors
    ///
    /// Returns an error if the Gmail API request fails.
    #[napi]
    pub async fn import_message(
        &self,
        raw_rfc822: String,
        deleted: bool,
    ) -> napi::Result<String> {
        let message = self
            .client
            .import_message(&raw_rfc822, deleted)
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(to_json_str(&message))
    }
}
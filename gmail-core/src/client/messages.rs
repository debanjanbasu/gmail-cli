//! Message operations

use base64::Engine;

use crate::error::{GmailError, Result};
use crate::models::*;

use super::build_email;

impl super::GmailClient {
    // ═══════════════════════════════════════════════════════════════════
    // Message Operations
    // ══════════════════════════════════════════════════════════════════

    /// Get message by ID
    pub async fn get_message(&self, message_id: &str, format: Option<&str>) -> Result<Message> {
        let mut request = self
            .http_client
            .get(self.api_url(&format!("users/me/messages/{}", message_id))?);

        if let Some(fmt) = format {
            request = request.query(&[("format", fmt)]);
        }

        let response = self.execute_with_retry(request).await?;
        Ok(response.json().await?)
    }

    /// Get message metadata only (lightweight)
    pub async fn get_message_metadata(&self, message_id: &str) -> Result<Message> {
        self.get_message(message_id, Some("metadata")).await
    }

    /// Get message raw (RFC 822)
    pub async fn get_message_raw(&self, message_id: &str) -> Result<String> {
        let response = self.execute_with_retry(
            self.http_client
                .get(self.api_url(&format!("users/me/messages/{}", message_id))?)
                .query(&[("format", "raw")])
        ).await?;
        let message: Message = response.json().await?;
        message.raw.ok_or_else(|| GmailError::NotFound("Raw message not available".into()))
    }

    /// Get attachment
    pub async fn get_attachment(&self, message_id: &str, attachment_id: &str) -> Result<Attachment> {
        let response = self.execute_with_retry(
            self.http_client
                .get(self.api_url(&format!("users/me/messages/{}/attachments/{}", message_id, attachment_id))?)
        ).await?;
        Ok(response.json().await?)
    }

    // ═══════════════════════════════════════════════════════════════════
    // Send Operations
    // ══════════════════════════════════════════════════════════════════

    /// Send email
    pub async fn send(&self, to: &str, subject: &str, body: &str) -> Result<Message> {
        let email = build_email(to, subject, body, None, None)?;
        let raw = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(email.as_bytes());
        
        let request = SendMessageRequest { raw, thread_id: None };
        
        let response = self.execute_with_retry(
            self.http_client
                .post(self.api_url("users/me/messages/send")?)
                .json(&request)
        ).await?;
        
        Ok(response.json().await?)
    }

/// Send email with CC/BCC
    pub async fn send_with_options(
        &self,
        to: &str,
        subject: &str,
        body: &str,
        cc: Option<&str>,
        bcc: Option<&str>,
    ) -> Result<Message> {
        let email = build_email(to, subject, body, cc, bcc)?;
        let raw = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(email.as_bytes());
        
        let request = SendMessageRequest { raw, thread_id: None };
        
        let response = self.execute_with_retry(
            self.http_client
                .post(self.api_url("users/me/messages/send")?)
                .json(&request)
        ).await?;
        
        Ok(response.json().await?)
    }

    /// Send email with attachments
    pub async fn send_with_attachments(
        &self,
        to: &str,
        subject: &str,
        body: &str,
        attachments: Vec<AttachmentData>,
        thread_id: Option<&str>,
    ) -> Result<Message> {
        let boundary = format!("----gmail_boundary_{}_{}", 
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0),
            rand::random::<u32>()
        );
        
        let mut message_parts = vec![
            format!("To: {}", to),
            format!("Subject: {}", subject),
            "MIME-Version: 1.0".to_string(),
            format!("Content-Type: multipart/mixed; boundary=\"{}\"", boundary),
            "".to_string(),
            format!("--{}", boundary),
            "Content-Type: text/plain; charset=\"UTF-8\"".to_string(),
            "".to_string(),
            body.to_string(),
        ];
        
        for att in attachments {
            let base64_content = base64::engine::general_purpose::STANDARD.encode(&att.content);
            message_parts.extend(vec![
                "".to_string(),
                format!("--{}", boundary),
                format!("Content-Type: {}", att.mime_type),
                "Content-Transfer-Encoding: base64".to_string(),
                format!("Content-Disposition: attachment; filename=\"{}\"", att.filename),
                "".to_string(),
                base64_content,
            ]);
        }
        
        message_parts.extend(vec!["".to_string(), format!("--{}--", boundary), "".to_string()]);
        let email = message_parts.join("\r\n");
        let raw = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(email.as_bytes());
        
        let request = SendMessageRequest { 
            raw, 
            thread_id: thread_id.map(|s| s.to_string()) 
        };
        
        let response = self.execute_with_retry(
            self.http_client
                .post(self.api_url("users/me/messages/send")?)
                .json(&request)
        ).await?;
        
        Ok(response.json().await?)
    }

    // ═══════════════════════════════════════════════════════════════════
    // Message Operations (Trash, Delete, etc.)
    // ══════════════════════════════════════════════════════════════════

    /// Trash message
    pub async fn trash_message(&self, message_id: &str) -> Result<Message> {
        let response = self.execute_with_retry(
            self.http_client
                .post(self.api_url(&format!("users/me/messages/{}/trash", message_id))?)
        ).await?;
        Ok(response.json().await?)
    }

    /// Untrash message
    pub async fn untrash_message(&self, message_id: &str) -> Result<Message> {
        let response = self.execute_with_retry(
            self.http_client
                .post(self.api_url(&format!("users/me/messages/{}/untrash", message_id))?)
        ).await?;
        Ok(response.json().await?)
    }

    /// Delete message permanently
    pub async fn delete_message(&self, message_id: &str) -> Result<()> {
        self.execute_with_retry(
            self.http_client
                .delete(self.api_url(&format!("users/me/messages/{}", message_id))?)
        ).await?;
        Ok(())
    }

    /// Batch delete messages
    pub async fn batch_delete_messages(&self, message_ids: &[String]) -> Result<()> {
        self.execute_with_retry(
            self.http_client
                .post(self.api_url("users/me/messages/batchDelete")?)
                .json(&serde_json::json!({ "ids": message_ids }))
        ).await?;
        Ok(())
    }

    // ═════════════════════════════════════════════════════════════════
    // Import
    // ═════════════════════════════════════════════════════════════════

    /// Import message from RFC 822
    pub async fn import_message(&self, raw_rfc822: &str, deleted: bool) -> Result<Message> {
        let raw = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw_rfc822.as_bytes());
        
        let request = ImportMessageRequest {
            raw,
            internal_date_source: Some("dateHeader".to_string()),
            deleted: Some(deleted),
            never_spam: Some(false),
            process_for_calendar: Some(false),
        };
        
        let response = self.execute_with_retry(
            self.http_client
                .post(self.api_url("users/me/messages/import")?)
                .json(&request)
        ).await?;
        Ok(response.json().await?)
    }
}
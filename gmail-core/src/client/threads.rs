//! Thread operations

use crate::error::Result;
use crate::models::*;

impl super::GmailClient {
    // ═══════════════════════════════════════════════════════════════════
    // Thread Operations
    // ═══════════════════════════════════════════════════════════════════

    /// Get thread by ID
    pub async fn get_thread(&self, thread_id: &str) -> Result<Thread> {
        let response = self.execute_with_retry(
            self.http_client
                .get(self.api_url(&format!("users/me/threads/{}", thread_id))?)
        ).await?;
        Ok(response.json().await?)
    }

    /// Modify thread labels
    pub async fn modify_thread_labels(
        &self,
        thread_id: &str,
        add_labels: &[String],
        remove_labels: &[String],
    ) -> Result<()> {
        let request = ModifyLabelsRequest {
            add_label_ids: add_labels.to_vec(),
            remove_label_ids: remove_labels.to_vec(),
        };
        
        self.execute_with_retry(
            self.http_client
                .post(self.api_url(&format!("users/me/threads/{}/modify", thread_id))?)
                .json(&request)
        ).await?;
        Ok(())
    }

    /// Trash thread
    pub async fn trash_thread(&self, thread_id: &str) -> Result<()> {
        self.execute_with_retry(
            self.http_client
                .post(self.api_url(&format!("users/me/threads/{}/trash", thread_id))?)
        ).await?;
        Ok(())
    }

    /// Untrash thread
    pub async fn untrash_thread(&self, thread_id: &str) -> Result<()> {
        self.execute_with_retry(
            self.http_client
                .post(self.api_url(&format!("users/me/threads/{}/untrash", thread_id))?)
        ).await?;
        Ok(())
    }

    /// Delete thread permanently
    pub async fn delete_thread(&self, thread_id: &str) -> Result<()> {
        self.execute_with_retry(
            self.http_client
                .delete(self.api_url(&format!("users/me/threads/{}", thread_id))?)
        ).await?;
        Ok(())
    }
}
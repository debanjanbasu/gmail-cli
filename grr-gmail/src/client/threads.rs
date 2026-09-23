//! Thread operations

use crate::models::*;
use grr_core::error::Result;

impl super::GmailClient {
    // â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
    // Thread Operations
    // â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

    /// Get thread by ID
    pub async fn get_thread(&self, thread_id: &str) -> Result<Thread> {
        let response = self
            .core
            .execute(
                self.core
                    .get(self.api_url(&format!("users/me/threads/{}", thread_id))?),
            )
            .await?;
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

        self.core
            .execute(
                self.core
                    .post(self.api_url(&format!("users/me/threads/{}/modify", thread_id))?)
                    .json(&request),
            )
            .await?;
        Ok(())
    }

    /// Trash thread
    pub async fn trash_thread(&self, thread_id: &str) -> Result<()> {
        self.core
            .execute(
                self.core
                    .post(self.api_url(&format!("users/me/threads/{}/trash", thread_id))?),
            )
            .await?;
        Ok(())
    }

    /// Untrash thread
    pub async fn untrash_thread(&self, thread_id: &str) -> Result<()> {
        self.core
            .execute(
                self.core
                    .post(self.api_url(&format!("users/me/threads/{}/untrash", thread_id))?),
            )
            .await?;
        Ok(())
    }

    /// Delete thread permanently
    pub async fn delete_thread(&self, thread_id: &str) -> Result<()> {
        self.core
            .execute(
                self.core
                    .delete(self.api_url(&format!("users/me/threads/{}", thread_id))?),
            )
            .await?;
        Ok(())
    }
}

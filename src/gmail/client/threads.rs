//! Thread operations

use crate::core::error::Result;
use crate::gmail::models::*;

impl super::GmailClient {
    // ═══════════════════════════════════════════════════════════════════
    // Thread Operations
    // ═══════════════════════════════════════════════════════════════════

    pub async fn list_threads(&self, query: &str, max_results: usize) -> Result<Vec<Thread>> {
        if max_results == 0 {
            return Ok(Vec::new());
        }

        let mut threads = Vec::new();
        let mut page_token = None;
        let batch_size = max_results.min(500);

        loop {
            let mut request = self
                .core
                .get(self.api_url("users/me/threads")?)
                .query(&[("q", query), ("maxResults", &batch_size.to_string())]);
            if let Some(token) = &page_token {
                request = request.query(&[("pageToken", token)]);
            }

            let response = self.core.execute(request).await?;
            let list: ThreadList = response.json().await?;
            threads.extend(list.threads);
            if threads.len() >= max_results {
                threads.truncate(max_results);
                break;
            }
            page_token = list.next_page_token;
            if page_token.is_none() {
                break;
            }
        }

        Ok(threads)
    }

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

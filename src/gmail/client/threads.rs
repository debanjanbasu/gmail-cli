//! Thread operations

use crate::core::error::Result;
use crate::core::http::QueryParams;
use crate::core::{Page, paginate};
use crate::gmail::models::*;

impl super::GmailClient {
    // ═══════════════════════════════════════════════════════════════════
    // Thread Operations
    // ═══════════════════════════════════════════════════════════════════

    pub async fn list_threads(&self, query: &str, max_results: usize) -> Result<Vec<Thread>> {
        if max_results == 0 {
            return Ok(Vec::new());
        }
        let batch_size = max_results.min(500);

        paginate(Some(max_results), move |page_token| async move {
            let params = QueryParams::new()
                .add("q", query)
                .add("maxResults", batch_size.to_string())
                .add_page_token(page_token.as_deref());
            let page: ThreadList = self
                .core
                .execute_json(params.apply(self.core.get(self.api_url("users/me/threads")?)))
                .await?;
            Ok(Page::new(page.threads, page.next_page_token))
        })
        .await
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

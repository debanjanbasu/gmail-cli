//! History and profile operations

use crate::core::error::Result;
use crate::gmail::models::*;

impl super::GmailClient {
    // ═══════════════════════════════════════════════════════════════════
    // History & Profile
    // ══════════════════════════════════════════════════════════════════

    /// Get history
    pub async fn get_history(
        &self,
        start_history_id: &str,
        label_id: Option<&str>,
        max_results: usize,
    ) -> Result<Vec<History>> {
        if max_results == 0 {
            return Ok(Vec::new());
        }

        let mut history = Vec::new();
        let mut page_token = None;
        let batch_size = max_results.min(500);

        loop {
            let mut request = self.core.get(self.api_url("users/me/history")?).query(&[
                ("startHistoryId", start_history_id),
                ("maxResults", &batch_size.to_string()),
            ]);
            if let Some(label) = label_id {
                request = request.query(&[("labelId", label)]);
            }
            if let Some(token) = &page_token {
                request = request.query(&[("pageToken", token)]);
            }

            let response = self.core.execute(request).await?;
            let list: HistoryList = response.json().await?;
            history.extend(list.history);
            if history.len() >= max_results {
                history.truncate(max_results);
                break;
            }
            page_token = list.next_page_token;
            if page_token.is_none() {
                break;
            }
        }

        Ok(history)
    }

    /// Get profile
    pub async fn get_profile(&self) -> Result<Profile> {
        let response = self
            .core
            .execute(self.core.get(self.api_url("users/me/profile")?))
            .await?;
        Ok(response.json().await?)
    }
}

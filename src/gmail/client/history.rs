//! History and profile operations

use crate::core::error::Result;
use crate::core::http::QueryParams;
use crate::core::{Page, paginate};
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
        let batch_size = max_results.min(500);

        paginate(Some(max_results), move |page_token| async move {
            let params = QueryParams::new()
                .add("startHistoryId", start_history_id)
                .add("maxResults", batch_size.to_string())
                .add_optional("labelId", label_id)
                .add_page_token(page_token.as_deref());
            let page: HistoryList = self
                .core
                .execute_json(params.apply(self.core.get(self.api_url("users/me/history")?)))
                .await?;
            Ok(Page::new(page.history, page.next_page_token))
        })
        .await
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

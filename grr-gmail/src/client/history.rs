//! History and profile operations

use crate::models::*;
use grr_core::error::Result;

impl super::GmailClient {
    // â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
    // History & Profile
    // â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

    /// Get history
    pub async fn get_history(
        &self,
        start_history_id: &str,
        label_id: Option<&str>,
        max_results: usize,
    ) -> Result<Vec<History>> {
        let mut request = self.core.get(self.api_url("users/me/history")?).query(&[
            ("startHistoryId", start_history_id),
            ("maxResults", &max_results.to_string()),
        ]);

        if let Some(label) = label_id {
            request = request.query(&[("labelId", label)]);
        }

        let response = self.core.execute(request).await?;
        let list_response: HistoryList = response.json().await?;
        Ok(list_response.history)
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

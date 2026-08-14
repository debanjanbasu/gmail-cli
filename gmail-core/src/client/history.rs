//! History and profile operations

use crate::error::Result;
use crate::models::*;

impl super::GmailClient {
    // ═══════════════════════════════════════════════════════════════════
    // History & Profile
    // ══════════════════════════════════════════════════════════════════

    /// Get history
    pub async fn get_history(&self, start_history_id: &str, label_id: Option<&str>, max_results: usize) -> Result<Vec<History>> {
        let mut request = self
            .http_client
            .get(self.api_url("users/me/history")?)
            .query(&[("startHistoryId", start_history_id), ("maxResults", &max_results.to_string())]);
        
        if let Some(label) = label_id {
            request = request.query(&[("labelId", label)]);
        }
        
        let response = self.execute_with_retry(request).await?;
        let list_response: ListResponse<History> = response.json().await?;
        Ok(list_response.items)
    }

    /// Get profile
    pub async fn get_profile(&self) -> Result<Profile> {
        let response = self.execute_with_retry(
            self.http_client
                .get(self.api_url("users/me/profile")?)
        ).await?;
        Ok(response.json().await?)
    }
}
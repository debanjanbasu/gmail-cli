//! Watch (push notifications) operations

use crate::error::Result;
use crate::models::*;

impl super::GmailClient {
    // ══════════════════════════════════════════════════════════════════
    // Watch (Push Notifications)
    // ════════════════════════════════════════════════════════════════

    /// Setup watch
    pub async fn watch(&self, topic_name: &str, label_ids: Option<Vec<String>>) -> Result<WatchResponse> {
        let request = WatchRequest {
            topic_name: topic_name.to_string(),
            label_ids,
            label_filter_action: None,
        };
        
        let response = self.execute_with_retry(
            self.http_client
                .post(self.api_url("users/me/watch")?)
                .json(&request)
        ).await?;
        Ok(response.json().await?)
    }

    /// Stop watch
    pub async fn stop_watch(&self) -> Result<()> {
        self.execute_with_retry(
            self.http_client
                .post(self.api_url("users/me/stop")?)
        ).await?;
        Ok(())
    }
}
//! Watch (push notifications) operations

use crate::models::*;
use grr_core::error::Result;

impl super::GmailClient {
    // â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
    // Watch (Push Notifications)
    // â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

    /// Setup watch
    pub async fn watch(
        &self,
        topic_name: &str,
        label_ids: Option<Vec<String>>,
    ) -> Result<WatchResponse> {
        let request = WatchRequest {
            topic_name: topic_name.to_string(),
            label_ids,
            label_filter_action: None,
        };

        let response = self
            .core
            .execute(
                self.core
                    .post(self.api_url("users/me/watch")?)
                    .json(&request),
            )
            .await?;
        Ok(response.json().await?)
    }

    /// Stop watch
    pub async fn stop_watch(&self) -> Result<()> {
        self.core
            .execute(self.core.post(self.api_url("users/me/stop")?))
            .await?;
        Ok(())
    }
}

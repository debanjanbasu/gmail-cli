//! Watch (push notifications) operations

use crate::core::error::Result;
use crate::gmail::models::*;

impl super::GmailClient {
    // ══════════════════════════════════════════════════════════════════
    // Watch (Push Notifications)
    // ════════════════════════════════════════════════════════════════

    /// Setup watch
    pub async fn watch(
        &self,
        topic_name: &str,
        label_ids: Option<Vec<String>>,
    ) -> Result<WatchResponse> {
        self.watch_with_options(topic_name, label_ids, None, None)
            .await
    }

    pub async fn watch_with_options(
        &self,
        topic_name: &str,
        label_ids: Option<Vec<String>>,
        label_filter_action: Option<String>,
        label_filter_behavior: Option<String>,
    ) -> Result<WatchResponse> {
        let request = WatchRequest {
            topic_name: topic_name.to_string(),
            label_ids,
            label_filter_action,
            label_filter_behavior,
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

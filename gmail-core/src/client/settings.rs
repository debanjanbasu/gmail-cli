//! Send-as (settings) operations

use crate::error::Result;
use crate::models::*;

impl super::GmailClient {
    // ═══════════════════════════════════════════════════════════════════
    // Send-as Operations
    // ═════════════════════════════════════════════════════════════════

    /// List send-as aliases
    pub async fn list_send_as(&self) -> Result<Vec<SendAs>> {
        let response = self
            .execute_with_retry(
                self.http_client
                    .get(self.api_url("users/me/settings/sendAs")?),
            )
            .await?;
        let list_response: SendAsList = response.json().await?;
        Ok(list_response.send_as)
    }

    /// Get send-as alias
    pub async fn get_send_as(&self, send_as_email: &str) -> Result<SendAs> {
        let response = self
            .execute_with_retry(
                self.http_client
                    .get(self.api_url(&format!("users/me/settings/sendAs/{}", send_as_email))?),
            )
            .await?;
        Ok(response.json().await?)
    }

    /// Create send-as alias
    pub async fn create_send_as(&self, options: CreateSendAsOptions) -> Result<SendAs> {
        let response = self
            .execute_with_retry(
                self.http_client
                    .post(self.api_url("users/me/settings/sendAs")?)
                    .json(&options),
            )
            .await?;
        Ok(response.json().await?)
    }

    /// Update send-as alias
    pub async fn update_send_as(
        &self,
        send_as_email: &str,
        options: UpdateSendAsOptions,
    ) -> Result<SendAs> {
        let response = self
            .execute_with_retry(
                self.http_client
                    .put(self.api_url(&format!("users/me/settings/sendAs/{}", send_as_email))?)
                    .json(&options),
            )
            .await?;
        Ok(response.json().await?)
    }

    /// Delete send-as alias
    pub async fn delete_send_as(&self, send_as_email: &str) -> Result<()> {
        self.execute_with_retry(
            self.http_client
                .delete(self.api_url(&format!("users/me/settings/sendAs/{}", send_as_email))?),
        )
        .await?;
        Ok(())
    }
}

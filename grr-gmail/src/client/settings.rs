//! Send-as (settings) operations

use crate::models::*;
use grr_core::error::Result;

impl super::GmailClient {
    // â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
    // Send-as Operations
    // â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

    /// List send-as aliases
    pub async fn list_send_as(&self) -> Result<Vec<SendAs>> {
        let response = self
            .core
            .execute(self.core.get(self.api_url("users/me/settings/sendAs")?))
            .await?;
        let list_response: SendAsList = response.json().await?;
        Ok(list_response.send_as)
    }

    /// Get send-as alias
    pub async fn get_send_as(&self, send_as_email: &str) -> Result<SendAs> {
        let response = self
            .core
            .execute(
                self.core
                    .get(self.api_url(&format!("users/me/settings/sendAs/{}", send_as_email))?),
            )
            .await?;
        Ok(response.json().await?)
    }

    /// Create send-as alias
    pub async fn create_send_as(&self, options: CreateSendAsOptions) -> Result<SendAs> {
        let response = self
            .core
            .execute(
                self.core
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
            .core
            .execute(
                self.core
                    .put(self.api_url(&format!("users/me/settings/sendAs/{}", send_as_email))?)
                    .json(&options),
            )
            .await?;
        Ok(response.json().await?)
    }

    /// Delete send-as alias
    pub async fn delete_send_as(&self, send_as_email: &str) -> Result<()> {
        self.core
            .execute(
                self.core
                    .delete(self.api_url(&format!("users/me/settings/sendAs/{}", send_as_email))?),
            )
            .await?;
        Ok(())
    }
}

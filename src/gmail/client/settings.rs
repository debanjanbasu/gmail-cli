//! Send-as (settings) operations

use crate::core::error::{GrrError, Result};
use crate::gmail::models::*;

impl super::GmailClient {
    // ═══════════════════════════════════════════════════════════════════
    // Send-as Operations
    // ═════════════════════════════════════════════════════════════════

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
    pub async fn patch_send_as(
        &self,
        send_as_email: &str,
        options: UpdateSendAsOptions,
    ) -> Result<SendAs> {
        let response = self
            .core
            .execute(
                self.core
                    .patch(self.api_url(&format!("users/me/settings/sendAs/{send_as_email}"))?)
                    .json(&options),
            )
            .await?;
        Ok(response.json().await?)
    }

    pub async fn delete_send_as(&self, send_as_email: &str) -> Result<()> {
        self.core
            .execute(
                self.core
                    .delete(self.api_url(&format!("users/me/settings/sendAs/{}", send_as_email))?),
            )
            .await?;
        Ok(())
    }

    pub async fn verify_send_as(&self, send_as_email: &str) -> Result<SendAs> {
        let response =
            self.core
                .execute(self.core.post(
                    self.api_url(&format!("users/me/settings/sendAs/{send_as_email}/verify"))?,
                ))
                .await?;
        Ok(response.json().await?)
    }

    pub async fn list_filters(&self) -> Result<Vec<Filter>> {
        let response = self
            .core
            .execute(self.core.get(self.api_url("users/me/settings/filters")?))
            .await?;
        let list: FilterList = response.json().await?;
        Ok(list.filter)
    }

    pub async fn get_filter(&self, filter_id: &str) -> Result<Filter> {
        let response = self
            .core
            .execute(
                self.core
                    .get(self.api_url(&format!("users/me/settings/filters/{filter_id}"))?),
            )
            .await?;
        Ok(response.json().await?)
    }

    pub async fn create_filter(&self, filter: Filter) -> Result<Filter> {
        let response = self
            .core
            .execute(
                self.core
                    .post(self.api_url("users/me/settings/filters")?)
                    .json(&filter),
            )
            .await?;
        Ok(response.json().await?)
    }

    pub async fn delete_filter(&self, filter_id: &str) -> Result<()> {
        self.core
            .execute(
                self.core
                    .delete(self.api_url(&format!("users/me/settings/filters/{filter_id}"))?),
            )
            .await?;
        Ok(())
    }

    pub async fn list_forwarding_addresses(&self) -> Result<Vec<ForwardingAddress>> {
        let response = self
            .core
            .execute(
                self.core
                    .get(self.api_url("users/me/settings/forwardingAddresses")?),
            )
            .await?;
        let list: ForwardingAddressList = response.json().await?;
        Ok(list.forwarding_addresses)
    }

    pub async fn get_forwarding_address(
        &self,
        forwarding_email: &str,
    ) -> Result<ForwardingAddress> {
        let response = self
            .core
            .execute(self.core.get(self.api_url(&format!(
                "users/me/settings/forwardingAddresses/{forwarding_email}"
            ))?))
            .await?;
        Ok(response.json().await?)
    }

    pub async fn create_forwarding_address(
        &self,
        forwarding_address: ForwardingAddress,
    ) -> Result<ForwardingAddress> {
        let response = self
            .core
            .execute(
                self.core
                    .post(self.api_url("users/me/settings/forwardingAddresses")?)
                    .json(&forwarding_address),
            )
            .await?;
        Ok(response.json().await?)
    }

    pub async fn delete_forwarding_address(&self, forwarding_email: &str) -> Result<()> {
        self.core
            .execute(self.core.delete(self.api_url(&format!(
                "users/me/settings/forwardingAddresses/{forwarding_email}"
            ))?))
            .await?;
        Ok(())
    }

    pub async fn get_auto_forwarding(&self) -> Result<AutoForwarding> {
        let response = self
            .core
            .execute(
                self.core
                    .get(self.api_url("users/me/settings/autoForwarding")?),
            )
            .await?;
        Ok(response.json().await?)
    }

    pub async fn update_auto_forwarding(&self, settings: AutoForwarding) -> Result<AutoForwarding> {
        validate_enum("disposition", settings.disposition.as_deref(), DISPOSITIONS)?;
        let response = self
            .core
            .execute(
                self.core
                    .put(self.api_url("users/me/settings/autoForwarding")?)
                    .json(&settings),
            )
            .await?;
        Ok(response.json().await?)
    }

    pub async fn get_pop_settings(&self) -> Result<PopSettings> {
        let response = self
            .core
            .execute(self.core.get(self.api_url("users/me/settings/pop")?))
            .await?;
        Ok(response.json().await?)
    }

    pub async fn update_pop_settings(&self, settings: PopSettings) -> Result<PopSettings> {
        validate_enum(
            "access window",
            settings.access_window.as_deref(),
            ACCESS_WINDOWS,
        )?;
        validate_enum("disposition", settings.disposition.as_deref(), DISPOSITIONS)?;
        let response = self
            .core
            .execute(
                self.core
                    .put(self.api_url("users/me/settings/pop")?)
                    .json(&settings),
            )
            .await?;
        Ok(response.json().await?)
    }

    pub async fn get_imap_settings(&self) -> Result<ImapSettings> {
        let response = self
            .core
            .execute(self.core.get(self.api_url("users/me/settings/imap")?))
            .await?;
        Ok(response.json().await?)
    }

    pub async fn update_imap_settings(&self, settings: ImapSettings) -> Result<ImapSettings> {
        validate_enum(
            "expunge behavior",
            settings.expunge_behavior.as_deref(),
            EXPUNGE_BEHAVIORS,
        )?;
        let response = self
            .core
            .execute(
                self.core
                    .put(self.api_url("users/me/settings/imap")?)
                    .json(&settings),
            )
            .await?;
        Ok(response.json().await?)
    }
}

const ACCESS_WINDOWS: &[&str] = &[
    "accessWindowUnspecified",
    "disabled",
    "fromNowOn",
    "allMail",
];

const DISPOSITIONS: &[&str] = &[
    "dispositionUnspecified",
    "leaveInInbox",
    "archive",
    "trash",
    "markRead",
];

const EXPUNGE_BEHAVIORS: &[&str] = &[
    "expungeBehaviorUnspecified",
    "archive",
    "trash",
    "deleteForever",
];

fn validate_enum(field: &str, value: Option<&str>, allowed: &[&str]) -> Result<()> {
    if let Some(value) = value
        && !allowed.contains(&value)
    {
        return Err(GrrError::InvalidArgument(format!(
            "invalid {field} `{value}`; allowed values: {}",
            allowed.join(", ")
        )));
    }
    Ok(())
}

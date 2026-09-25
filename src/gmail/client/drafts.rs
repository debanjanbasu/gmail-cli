//! Draft operations

use base64::Engine;

use crate::core::error::Result;
use crate::gmail::models::*;

use super::build_email;

impl super::GmailClient {
    // ═══════════════════════════════════════════════════════════════════
    // Draft Operations
    // ══════════════════════════════════════════════════════════════════

    /// Create draft
    pub async fn create_draft(&self, to: &str, subject: &str, body: &str) -> Result<Draft> {
        let request = draft_request(to, subject, body)?;

        let response = self
            .core
            .execute(
                self.core
                    .post(self.api_url("users/me/drafts")?)
                    .json(&request),
            )
            .await?;

        Ok(response.json().await?)
    }

    /// List drafts
    pub async fn list_drafts(&self, max_results: usize) -> Result<Vec<Draft>> {
        let response = self
            .core
            .execute(
                self.core
                    .get(self.api_url("users/me/drafts")?)
                    .query(&[("maxResults", &max_results.to_string())]),
            )
            .await?;

        let list_response: DraftList = response.json().await?;
        Ok(list_response.drafts)
    }

    /// Get draft
    pub async fn get_draft(&self, draft_id: &str) -> Result<Draft> {
        let response = self
            .core
            .execute(
                self.core
                    .get(self.api_url(&format!("users/me/drafts/{}", draft_id))?),
            )
            .await?;
        Ok(response.json().await?)
    }

    /// Send draft
    pub async fn send_draft(&self, draft_id: &str) -> Result<Message> {
        let response = self
            .core
            .execute(
                self.core
                    .post(self.api_url(&format!("users/me/drafts/{}/send", draft_id))?),
            )
            .await?;
        Ok(response.json().await?)
    }

    /// Update draft
    pub async fn update_draft(
        &self,
        draft_id: &str,
        to: &str,
        subject: &str,
        body: &str,
    ) -> Result<Draft> {
        let request = draft_request(to, subject, body)?;

        let response = self
            .core
            .execute(
                self.core
                    .put(self.api_url(&format!("users/me/drafts/{}", draft_id))?)
                    .json(&request),
            )
            .await?;

        Ok(response.json().await?)
    }

    /// Delete draft
    pub async fn delete_draft(&self, draft_id: &str) -> Result<()> {
        self.core
            .execute(
                self.core
                    .delete(self.api_url(&format!("users/me/drafts/{}", draft_id))?),
            )
            .await?;
        Ok(())
    }
}

/// Build the JSON body shared by draft create and update from a plain
/// (to, subject, body) triple.
fn draft_request(to: &str, subject: &str, body: &str) -> Result<CreateDraftRequest> {
    let email = build_email(to, subject, body, None, None)?;
    let raw = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(email.as_bytes());
    Ok(CreateDraftRequest {
        message: DraftMessage {
            raw,
            thread_id: None,
        },
    })
}

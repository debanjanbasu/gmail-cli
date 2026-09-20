//! Label operations

use crate::error::Result;
use crate::models::*;

impl super::GmailClient {
    // ═══════════════════════════════════════════════════════════════════
    // Label Operations
    // ══════════════════════════════════════════════════════════════════

    /// List labels
    pub async fn list_labels(&self) -> Result<Vec<Label>> {
        let response = self
            .execute_with_retry(self.http_client.get(self.api_url("users/me/labels")?))
            .await?;
        let list_response: LabelList = response.json().await?;
        Ok(list_response.labels)
    }

    /// Get label
    pub async fn get_label(&self, label_id: &str) -> Result<Label> {
        let response = self
            .execute_with_retry(
                self.http_client
                    .get(self.api_url(&format!("users/me/labels/{}", label_id))?),
            )
            .await?;
        Ok(response.json().await?)
    }

    /// Create label with options
    pub async fn create_label(&self, name: &str, options: CreateLabelOptions) -> Result<Label> {
        let request = CreateLabelRequest {
            name: name.to_string(),
            label_list_visibility: options.label_list_visibility,
            message_list_visibility: options.message_list_visibility,
            color: options.color,
        };

        let response = self
            .execute_with_retry(
                self.http_client
                    .post(self.api_url("users/me/labels")?)
                    .json(&request),
            )
            .await?;
        Ok(response.json().await?)
    }

    /// Update label
    pub async fn update_label(&self, label_id: &str, options: UpdateLabelOptions) -> Result<Label> {
        let request = UpdateLabelRequest {
            name: options.name,
            label_list_visibility: options.label_list_visibility,
            message_list_visibility: options.message_list_visibility,
            color: options.color,
        };

        let response = self
            .execute_with_retry(
                self.http_client
                    .put(self.api_url(&format!("users/me/labels/{}", label_id))?)
                    .json(&request),
            )
            .await?;
        Ok(response.json().await?)
    }

    /// Modify labels on message
    pub async fn modify_labels(
        &self,
        message_id: &str,
        add_labels: &[String],
        remove_labels: &[String],
    ) -> Result<Message> {
        let request = ModifyLabelsRequest {
            add_label_ids: add_labels.to_vec(),
            remove_label_ids: remove_labels.to_vec(),
        };

        let response = self
            .execute_with_retry(
                self.http_client
                    .post(self.api_url(&format!("users/me/messages/{}/modify", message_id))?)
                    .json(&request),
            )
            .await?;
        Ok(response.json().await?)
    }

    /// Batch modify labels
    pub async fn batch_modify_labels(
        &self,
        message_ids: &[String],
        add_labels: &[String],
        remove_labels: &[String],
    ) -> Result<()> {
        let request = BatchModifyLabelsRequest {
            ids: message_ids.to_vec(),
            add_label_ids: add_labels.to_vec(),
            remove_label_ids: remove_labels.to_vec(),
        };

        self.execute_with_retry(
            self.http_client
                .post(self.api_url("users/me/messages/batchModify")?)
                .json(&request),
        )
        .await?;
        Ok(())
    }

    /// Delete label
    pub async fn delete_label(&self, label_id: &str) -> Result<()> {
        self.execute_with_retry(
            self.http_client
                .delete(self.api_url(&format!("users/me/labels/{}", label_id))?),
        )
        .await?;
        Ok(())
    }
}

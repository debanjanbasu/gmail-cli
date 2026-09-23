//! Google Forms API response models deserialized with serde.
//!
//! Lenient by design: every field is optional, unknown fields are
//! ignored, and nested shapes mirror the REST wire format exactly
//! (answers ride `textAnswers.answers[].value`, per the v1 discovery
//! document).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A Form resource (forms.get)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Form {
    pub form_id: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub items: Vec<FormItem>,
    pub linked_sheet_id: Option<String>,
    pub published_url: Option<String>,
    pub responder_uri: Option<String>,
    pub revision_id: Option<String>,
}

/// An item on a form page (the API's `Item`: a question, image, video,
/// or page break; only question items carry typed fields here)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FormItem {
    pub item_id: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub question_item: Option<QuestionItem>,
}

/// A question item on a form
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionItem {
    pub question: Option<Question>,
}

/// A question on a form
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Question {
    pub question_id: Option<String>,
    pub required: Option<bool>,
    pub choice_question: Option<ChoiceQuestion>,
    pub text_question: Option<TextQuestion>,
}

/// A multiple-choice question (RADIO, CHECKBOX, DROP_DOWN)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChoiceQuestion {
    pub options: Option<Vec<ChoiceOption>>,
}

/// One selectable choice (the API's `Option`)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChoiceOption {
    pub value: Option<String>,
}

/// A free-text question (short answer or paragraph)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextQuestion {
    pub paragraph: Option<bool>,
}

/// A single submitted response to a form
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FormResponse {
    pub response_id: Option<String>,
    pub create_time: Option<String>,
    pub last_submitted_time: Option<String>,
    /// Answers keyed by question ID
    pub answers: Option<HashMap<String, Answer>>,
    pub respondent_email: Option<String>,
}

/// The submitted answer to a single question
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Answer {
    pub question_id: Option<String>,
    pub text_answers: Option<TextAnswers>,
}

/// Text answers to a question (multiple values for CHECKBOX questions)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextAnswers {
    #[serde(default)]
    pub answers: Vec<TextAnswer>,
}

/// One answer value as text (the rendering depends on the question type)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextAnswer {
    pub value: Option<String>,
}

/// Envelope of forms.responses.list (one result page)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListFormResponsesResponse {
    pub responses: Option<Vec<FormResponse>>,
    pub next_page_token: Option<String>,
}

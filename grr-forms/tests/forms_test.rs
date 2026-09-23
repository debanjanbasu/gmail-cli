//! Wiremock tests for the Forms client over the shared HTTP core.

use grr_core::{GoogleAuth, GrrConfig, TokenStorage};
use grr_forms::{FormsClient, FormsClientBuilder};
use wiremock::matchers::{method, path, query_param, query_param_is_missing};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn get_form_parses_items_and_questions() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/forms/FORM123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "formId": "FORM123",
            "title": "Team retrospection",
            "description": "Quarterly retro",
            "items": [
                {
                    "itemId": "item_1",
                    "title": "Favorite color",
                    "description": "Pick one",
                    "questionItem": {
                        "question": {
                            "questionId": "question_1",
                            "required": true,
                            "choiceQuestion": {
                                "type": "RADIO",
                                "options": [
                                    { "value": "Red", "isOther": false },
                                    { "value": "Blue" }
                                ]
                            }
                        }
                    }
                },
                {
                    "itemId": "item_2",
                    "title": "Why that color?",
                    "questionItem": {
                        "question": {
                            "questionId": "question_2",
                            "required": false,
                            "textQuestion": { "paragraph": true }
                        }
                    }
                }
            ],
            "linkedSheetId": "SHEET123",
            "publishedUrl": "https://docs.google.com/forms/d/e/FORM123/viewform",
            "responderUri": "https://docs.google.com/forms/d/e/FORM123/viewform",
            "revisionId": "42"
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let form = client.get_form("FORM123").await.unwrap();

    assert_eq!(form.form_id.as_deref(), Some("FORM123"));
    assert_eq!(form.title.as_deref(), Some("Team retrospection"));
    assert_eq!(form.description.as_deref(), Some("Quarterly retro"));
    assert_eq!(form.linked_sheet_id.as_deref(), Some("SHEET123"));
    assert_eq!(
        form.published_url.as_deref(),
        Some("https://docs.google.com/forms/d/e/FORM123/viewform")
    );
    assert_eq!(
        form.responder_uri.as_deref(),
        Some("https://docs.google.com/forms/d/e/FORM123/viewform")
    );
    assert_eq!(form.revision_id.as_deref(), Some("42"));

    assert_eq!(form.items.len(), 2);
    let first = &form.items[0];
    assert_eq!(first.item_id.as_deref(), Some("item_1"));
    assert_eq!(first.title.as_deref(), Some("Favorite color"));
    assert_eq!(first.description.as_deref(), Some("Pick one"));

    let question = first
        .question_item
        .as_ref()
        .and_then(|qi| qi.question.as_ref())
        .unwrap();
    assert_eq!(question.question_id.as_deref(), Some("question_1"));
    assert_eq!(question.required, Some(true));
    let choice = question.choice_question.as_ref().unwrap();
    let options = choice.options.as_ref().unwrap();
    assert_eq!(options.len(), 2);
    assert_eq!(options[0].value.as_deref(), Some("Red"));
    assert_eq!(options[1].value.as_deref(), Some("Blue"));

    let second = &form.items[1];
    assert_eq!(second.item_id.as_deref(), Some("item_2"));
    let second_question = second
        .question_item
        .as_ref()
        .and_then(|qi| qi.question.as_ref())
        .unwrap();
    assert_eq!(second_question.question_id.as_deref(), Some("question_2"));
    assert_eq!(second_question.required, Some(false));
    assert!(second_question.choice_question.is_none());
    assert_eq!(
        second_question
            .text_question
            .as_ref()
            .and_then(|t| t.paragraph),
        Some(true)
    );
}

#[tokio::test]
async fn list_responses_follows_page_tokens_and_parses_answers() {
    let server = MockServer::start().await;

    // Page 1: no pageToken in the query string yet.
    Mock::given(method("GET"))
        .and(path("/forms/FORM123/responses"))
        .and(query_param_is_missing("pageToken"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "responses": [
                {
                    "responseId": "resp1",
                    "createTime": "2026-09-20T10:00:00Z",
                    "lastSubmittedTime": "2026-09-20T10:01:00Z",
                    "respondentEmail": "alice@example.com",
                    "answers": {
                        "question_1": {
                            "questionId": "question_1",
                            "textAnswers": { "answers": [ { "value": "Blue" } ] }
                        }
                    }
                }
            ],
            "nextPageToken": "page-2"
        })))
        .mount(&server)
        .await;

    // Page 2: fetched with pageToken=page-2, no further pages.
    Mock::given(method("GET"))
        .and(path("/forms/FORM123/responses"))
        .and(query_param("pageToken", "page-2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "responses": [
                {
                    "responseId": "resp2",
                    "createTime": "2026-09-21T09:00:00Z",
                    "lastSubmittedTime": "2026-09-21T09:02:00Z",
                    "answers": {
                        "question_1": {
                            "textAnswers": {
                                "answers": [ { "value": "Red" }, { "value": "Green" } ]
                            }
                        },
                        "question_2": {
                            "textAnswers": { "answers": [ { "value": "Because." } ] }
                        }
                    }
                }
            ]
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let responses = client.list_responses("FORM123", Some(2)).await.unwrap();

    let ids: Vec<&str> = responses
        .iter()
        .map(|r| r.response_id.as_deref().unwrap_or_default())
        .collect();
    assert_eq!(ids, ["resp1", "resp2"]);

    assert_eq!(
        responses[0].respondent_email.as_deref(),
        Some("alice@example.com")
    );
    assert_eq!(
        responses[0].create_time.as_deref(),
        Some("2026-09-20T10:00:00Z")
    );
    assert_eq!(
        responses[0].last_submitted_time.as_deref(),
        Some("2026-09-20T10:01:00Z")
    );

    // Answers map is keyed by question ID; the value chain is
    // Answer -> textAnswers -> answers[] -> value (per the v1 wire format).
    let answers = responses[0].answers.as_ref().unwrap();
    let answer = answers.get("question_1").unwrap();
    assert_eq!(answer.question_id.as_deref(), Some("question_1"));
    let text = answer
        .text_answers
        .as_ref()
        .and_then(|t| t.answers.first())
        .and_then(|a| a.value.as_deref());
    assert_eq!(text, Some("Blue"));

    // CHECKBOX-style answers carry multiple values in the same Answer.
    let answers = responses[1].answers.as_ref().unwrap();
    let multi = answers.get("question_1").unwrap();
    let values: Vec<&str> = multi
        .text_answers
        .as_ref()
        .unwrap()
        .answers
        .iter()
        .filter_map(|a| a.value.as_deref())
        .collect();
    assert_eq!(values, ["Red", "Green"]);
    let free_text = answers
        .get("question_2")
        .unwrap()
        .text_answers
        .as_ref()
        .and_then(|t| t.answers.first())
        .and_then(|a| a.value.as_deref());
    assert_eq!(free_text, Some("Because."));
}

#[tokio::test]
async fn list_responses_handles_empty_envelope() {
    let server = MockServer::start().await;
    // A bare {} (no "responses" key, no nextPageToken) is a valid final
    // page: the lenient model must parse it as "no more responses".
    Mock::given(method("GET"))
        .and(path("/forms/FORM123/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let responses = client.list_responses("FORM123", Some(100)).await.unwrap();
    assert!(responses.is_empty());
}

#[tokio::test]
async fn form_id_normalization_hits_the_same_path() {
    let server = MockServer::start().await;
    // Exactly two hits: both the bare ID and the full resource name must
    // resolve to the same /forms/ABC123 path.
    Mock::given(method("GET"))
        .and(path("/forms/ABC123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "formId": "ABC123",
            "title": "Either spelling works",
            "items": []
        })))
        .expect(2)
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let by_id = client.get_form("ABC123").await.unwrap();
    let by_resource_name = client.get_form("forms/ABC123").await.unwrap();
    assert_eq!(by_id.title.as_deref(), Some("Either spelling works"));
    assert_eq!(
        by_resource_name.title.as_deref(),
        Some("Either spelling works")
    );
}

/// Build a client pointed at the mock server with a pre-authed, in-memory
/// token so no OAuth flow or token-endpoint round-trip ever happens.
/// Both h3 and h2 prior-knowledge are disabled: wiremock speaks HTTP/1.1.
async fn test_client(base: &str) -> FormsClient {
    let config = GrrConfig::default();
    let storage = TokenStorage {
        access_token: "test-token".into(),
        refresh_token: None,
        expires_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 3600,
        token_type: "Bearer".into(),
        scope: "test-scope".into(),
    };
    // Keep token persistence (if a test ever refreshes) off the real
    // credential store: it goes to a tempdir file instead.
    let auth = GoogleAuth::with_token(config.oauth.clone(), storage)
        .await
        .unwrap()
        .with_token_path(tempfile::tempdir().unwrap().keep());
    FormsClientBuilder::new()
        .auth(auth)
        .base_url(base.parse().unwrap())
        .build()
        .await
        .unwrap()
}

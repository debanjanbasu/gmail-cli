use grr_core::{GoogleAuth, GrrConfig, TokenStorage};
use grr_people::{
    EmailAddress, ListConnectionsOptions, Name, PeopleClient, PeopleClientBuilder, Person,
    PhoneNumber,
};
use wiremock::matchers::{body_json, method, path, query_param, query_param_is_missing};
use wiremock::{Mock, MockServer, ResponseTemplate};

const READ_PERSON_FIELDS: &str = "names,emailAddresses,phoneNumbers,organizations,photos";

#[tokio::test]
async fn list_connections_follows_all_pages() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/people/me/connections"))
        .and(query_param_is_missing("pageToken"))
        .and(query_param("queryName", "ada"))
        .and(query_param("pageSize", "2"))
        .and(query_param("personFields", READ_PERSON_FIELDS))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "connections": [
                { "resourceName": "people/p1", "names": [ { "givenName": "Ada" } ] },
                { "resourceName": "people/p2", "names": [ { "givenName": "Alan" } ] }
            ],
            "nextPageToken": "page-2"
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/people/me/connections"))
        .and(query_param("pageToken", "page-2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "connections": [
                { "resourceName": "people/p3", "names": [ { "givenName": "Grace" } ] }
            ],
            "totalItems": 3
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let people = client
        .list_connections(ListConnectionsOptions {
            page_size: 2,
            query_name: Some("ada".into()),
            max_results: 0,
        })
        .await
        .unwrap();

    let names: Vec<&str> = people
        .iter()
        .filter_map(|p| {
            p.names
                .as_ref()
                .and_then(|names| names.first())
                .and_then(|n| n.given_name.as_deref())
        })
        .collect();
    assert_eq!(names, vec!["Ada", "Alan", "Grace"]);
}

#[tokio::test]
async fn get_person_fetches_requested_resource() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/people/c123"))
        .and(query_param("personFields", READ_PERSON_FIELDS))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "resourceName": "people/c123",
            "etag": "etag-1",
            "names": [ { "displayName": "Ada Lovelace", "givenName": "Ada", "familyName": "Lovelace" } ],
            "emailAddresses": [ { "value": "ada@example.com", "type": "home" } ],
            "phoneNumbers": [ { "value": "+1 555 0100" } ],
            "organizations": [ { "name": "Analytical Engine Co", "title": "Mathematician" } ],
            "photos": [ { "url": "https://example.com/ada.png" } ]
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let person = client.get_person("people/c123").await.unwrap();

    assert_eq!(person.resource_name.as_deref(), Some("people/c123"));
    assert_eq!(person.etag.as_deref(), Some("etag-1"));
    let name = person
        .names
        .as_ref()
        .and_then(|names| names.first())
        .unwrap();
    assert_eq!(name.given_name.as_deref(), Some("Ada"));
    assert_eq!(name.family_name.as_deref(), Some("Lovelace"));
    let email = person
        .email_addresses
        .as_ref()
        .and_then(|emails| emails.first())
        .unwrap();
    assert_eq!(email.value.as_deref(), Some("ada@example.com"));
    assert_eq!(email.r#type.as_deref(), Some("home"));
}

#[tokio::test]
async fn search_contacts_uses_prefix_search_endpoint() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/people:searchContacts"))
        .and(query_param("query", "ada"))
        .and(query_param("readMask", "names,emailAddresses,phoneNumbers"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [
                { "person": {
                    "resourceName": "people/c77",
                    "names": [ { "givenName": "Ada" } ],
                    "emailAddresses": [ { "value": "ada@example.com" } ]
                } }
            ]
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let people = client.search_contacts("ada").await.unwrap();

    assert_eq!(people.len(), 1);
    assert_eq!(people[0].resource_name.as_deref(), Some("people/c77"));
}

#[tokio::test]
async fn create_contact_posts_person_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/people:createContact"))
        .and(body_json(serde_json::json!({
            "names": [ { "givenName": "Ada", "familyName": "Lovelace" } ],
            "emailAddresses": [ { "value": "ada@example.com" } ],
            "phoneNumbers": [ { "value": "+1 555 0100" } ]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "resourceName": "people/c9001",
            "names": [ { "givenName": "Ada", "familyName": "Lovelace" } ],
            "emailAddresses": [ { "value": "ada@example.com" } ],
            "phoneNumbers": [ { "value": "+1 555 0100" } ]
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let created = client
        .create_contact(Person {
            names: Some(vec![Name {
                given_name: Some("Ada".into()),
                family_name: Some("Lovelace".into()),
                ..Name::default()
            }]),
            email_addresses: Some(vec![EmailAddress {
                value: Some("ada@example.com".into()),
                ..EmailAddress::default()
            }]),
            phone_numbers: Some(vec![PhoneNumber {
                value: Some("+1 555 0100".into()),
                ..PhoneNumber::default()
            }]),
            ..Person::default()
        })
        .await
        .unwrap();

    assert_eq!(created.resource_name.as_deref(), Some("people/c9001"));
}

#[tokio::test]
async fn update_contact_patches_with_update_person_fields() {
    let server = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path("/people/c123:updateContact"))
        .and(query_param(
            "updatePersonFields",
            "names,emailAddresses,phoneNumbers",
        ))
        .and(body_json(serde_json::json!({
            "resourceName": "people/c123",
            "etag": "etag-1",
            "names": [ { "givenName": "Ada", "familyName": "Byron" } ],
            "emailAddresses": [ { "value": "ada@example.com" } ],
            "phoneNumbers": [ { "value": "+1 555 0100" } ]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "resourceName": "people/c123",
            "etag": "etag-2",
            "names": [ { "givenName": "Ada", "familyName": "Byron" } ],
            "emailAddresses": [ { "value": "ada@example.com" } ],
            "phoneNumbers": [ { "value": "+1 555 0100" } ]
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let updated = client
        .update_contact(
            "people/c123",
            Person {
                resource_name: Some("people/c123".into()),
                etag: Some("etag-1".into()),
                names: Some(vec![Name {
                    given_name: Some("Ada".into()),
                    family_name: Some("Byron".into()),
                    ..Name::default()
                }]),
                email_addresses: Some(vec![EmailAddress {
                    value: Some("ada@example.com".into()),
                    ..EmailAddress::default()
                }]),
                phone_numbers: Some(vec![PhoneNumber {
                    value: Some("+1 555 0100".into()),
                    ..PhoneNumber::default()
                }]),
                ..Person::default()
            },
            &["names", "emailAddresses", "phoneNumbers"],
        )
        .await
        .unwrap();

    assert_eq!(updated.etag.as_deref(), Some("etag-2"));
}

#[tokio::test]
async fn update_contact_rejects_empty_field_mask() {
    let server = MockServer::start().await;
    let client = test_client(&server.uri()).await;
    let result = client
        .update_contact("people/c123", Person::default(), &[])
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn delete_contact_succeeds_on_204() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/people/c123:deleteContact"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    client.delete_contact("people/c123").await.unwrap();
}

/// Build a client pointed at the mock server with a pre-authed, in-memory
/// token and a tempdir token path, so neither OAuth flows nor token
/// storage can touch the user's real cached credential. Both h3 and h2
/// prior-knowledge are disabled: wiremock speaks HTTP/1.1.
async fn test_client(base: &str) -> PeopleClient {
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
    let dir = tempfile::tempdir().unwrap();
    let auth = GoogleAuth::with_token(config.oauth.clone(), storage)
        .await
        .unwrap()
        .with_token_path(dir.path().join("token.json"));
    PeopleClientBuilder::new()
        .auth(auth)
        .base_url(base.parse().unwrap())
        .build()
        .await
        .unwrap()
}

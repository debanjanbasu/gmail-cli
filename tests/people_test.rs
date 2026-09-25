#![cfg(feature = "people")]
use grr_cli::core::{GoogleAuth, GrrConfig, TokenStorage};
use grr_cli::people::{
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

#[tokio::test]
async fn list_contact_groups_uses_group_fields() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/contactGroups"))
        .and(query_param(
            "groupFields",
            "clientData,groupType,memberCount,metadata,name",
        ))
        .and(query_param("pageSize", "1000"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "contactGroups": [{
                "resourceName": "contactGroups/friends",
                "formattedName": "Friends",
                "name": "Friends",
                "memberCount": 2,
                "groupType": "USER_CONSUMER"
            }],
            "totalItems": 1
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let groups = client.list_contact_groups().await.unwrap();

    assert_eq!(groups.len(), 1);
    assert_eq!(
        groups[0].resource_name.as_deref(),
        Some("contactGroups/friends")
    );
    assert_eq!(groups[0].name.as_deref(), Some("Friends"));
    assert_eq!(groups[0].member_count, Some(2));
    assert_eq!(groups[0].group_type.as_deref(), Some("USER_CONSUMER"));
}

#[tokio::test]
async fn get_contact_group_uses_resource_name() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/contactGroups/friends"))
        .and(query_param(
            "groupFields",
            "clientData,groupType,memberCount,metadata,name",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "resourceName": "contactGroups/friends",
            "formattedName": "Friends",
            "name": "Friends",
            "memberCount": 2,
            "groupType": "USER_CONSUMER"
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let group = client
        .get_contact_group("contactGroups/friends")
        .await
        .unwrap();

    assert_eq!(group.name.as_deref(), Some("Friends"));
}

#[tokio::test]
async fn create_contact_group_posts_wrapped_group() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/contactGroups"))
        .and(body_json(serde_json::json!({
            "contactGroup": { "name": "Friends" },
            "readGroupFields": "clientData,groupType,metadata,name"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "resourceName": "contactGroups/friends",
            "name": "Friends",
            "groupType": "USER_CONSUMER"
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let group = client.create_contact_group("Friends").await.unwrap();

    assert_eq!(
        group.resource_name.as_deref(),
        Some("contactGroups/friends")
    );
}

#[tokio::test]
async fn update_contact_group_puts_name_mask() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/contactGroups/friends"))
        .and(body_json(serde_json::json!({
            "contactGroup": {
                "resourceName": "contactGroups/friends",
                "name": "Close Friends"
            },
            "updateGroupFields": "name",
            "readGroupFields": "clientData,groupType,memberCount,metadata,name"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "resourceName": "contactGroups/friends",
            "name": "Close Friends"
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let group = client
        .update_contact_group("contactGroups/friends", "Close Friends")
        .await
        .unwrap();

    assert_eq!(group.name.as_deref(), Some("Close Friends"));
}

#[tokio::test]
async fn delete_contact_group_uses_delete_verb() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/contactGroups/friends"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    client
        .delete_contact_group("contactGroups/friends")
        .await
        .unwrap();
}

#[tokio::test]
async fn modify_contact_group_members_posts_official_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/contactGroups/friends/members:modify"))
        .and(body_json(serde_json::json!({
            "resourceNamesToAdd": ["people/1", "people/2"]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "notFoundResourceNames": []
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let response = client
        .modify_contact_group_members(
            "contactGroups/friends",
            &["people/1".to_owned(), "people/2".to_owned()],
            &[],
        )
        .await
        .unwrap();

    assert!(response.not_found_resource_names.unwrap().is_empty());
}

#[tokio::test]
async fn get_people_repeats_resource_names_query_parameter() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/people:batchGet"))
        .and(query_param("resourceNames", "people/1"))
        .and(query_param("resourceNames", "people/2"))
        .and(query_param("personFields", "names,emailAddresses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "responses": [
                {
                    "resourceName": "people/1",
                    "httpStatusCode": 200,
                    "person": { "resourceName": "people/1" }
                },
                {
                    "requestedResourceName": "people/2",
                    "httpStatusCode": 200,
                    "person": { "resourceName": "people/2" }
                }
            ]
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let response = client
        .get_people(
            &["people/1".to_owned(), "people/2".to_owned()],
            "names,emailAddresses",
        )
        .await
        .unwrap();

    let requests = server.received_requests().await.unwrap();
    let resource_names: Vec<String> = requests[0]
        .url
        .query_pairs()
        .filter(|(key, _)| key == "resourceNames")
        .map(|(_, value)| value.into_owned())
        .collect();
    assert_eq!(resource_names, vec!["people/1", "people/2"]);
    assert_eq!(response.responses.unwrap().len(), 2);
}

#[tokio::test]
async fn list_other_contacts_requires_read_mask() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/otherContacts"))
        .and(query_param("readMask", "names,emailAddresses"))
        .and(query_param("pageSize", "2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "otherContacts": [
                { "resourceName": "otherContacts/o1" },
                { "resourceName": "otherContacts/o2" }
            ],
            "totalSize": 2
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let contacts = client.list_other_contacts(2).await.unwrap();

    assert_eq!(contacts.len(), 2);
    assert_eq!(
        contacts[0].resource_name.as_deref(),
        Some("otherContacts/o1")
    );
}

#[tokio::test]
async fn copy_other_contact_posts_copy_mask_and_source() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/otherContacts/o123:copyOtherContactToMyContactsGroup",
        ))
        .and(body_json(serde_json::json!({
            "copyMask": "names,emailAddresses,phoneNumbers",
            "sources": ["READ_SOURCE_TYPE_CONTACT"]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "resourceName": "people/c123",
            "names": [{ "displayName": "Ada Lovelace" }]
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let person = client
        .copy_other_contact_to_my_contacts_group("otherContacts/o123")
        .await
        .unwrap();

    assert_eq!(person.resource_name.as_deref(), Some("people/c123"));
}

#[tokio::test]
async fn list_photos_uses_people_get_with_photo_fields() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/people/c123"))
        .and(query_param("personFields", "photos"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "resourceName": "people/c123",
            "photos": [{
                "url": "https://example.com/ada.png",
                "default": true,
                "metadata": {
                    "primary": true,
                    "sourcePrimary": true,
                    "verified": true
                }
            }]
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let photos = client.list_photos("people/c123").await.unwrap();

    assert_eq!(photos.len(), 1);
    assert_eq!(
        photos[0].url.as_deref(),
        Some("https://example.com/ada.png")
    );
    assert_eq!(photos[0].default, Some(true));
    assert_eq!(
        photos[0]
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.primary),
        Some(true)
    );
}

#[tokio::test]
async fn update_contact_photo_patches_base64_bytes() {
    let server = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path("/people/c123:updateContactPhoto"))
        .and(body_json(serde_json::json!({
            "photoBytes": "cGhvdG8tYnl0ZXM=",
            "personFields": "photos",
            "sources": ["READ_SOURCE_TYPE_CONTACT"]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "person": {
                "resourceName": "people/c123",
                "photos": [{
                    "url": "https://example.com/new.png",
                    "metadata": { "primary": true }
                }]
            }
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let response = client
        .update_contact_photo("people/c123", "cGhvdG8tYnl0ZXM=")
        .await
        .unwrap();
    let photos = response.person.unwrap().photos.unwrap();

    assert_eq!(
        photos[0].url.as_deref(),
        Some("https://example.com/new.png")
    );
}

#[tokio::test]
async fn delete_contact_photo_uses_delete_custom_verb() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/people/c123:deleteContactPhoto"))
        .and(query_param("personFields", "photos"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "person": {
                "resourceName": "people/c123",
                "photos": []
            }
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let response = client.delete_contact_photo("people/c123").await.unwrap();
    let photos = response.person.unwrap().photos.unwrap();

    assert!(photos.is_empty());
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

use std::future::Future;

use tracing::info;
use url::Url;

use crate::core::auth::{DeviceAuthChallenge, GoogleAuth, TokenStorage};
use crate::core::error::{GrrError, Result};
use crate::core::http::{HttpCore, TransportInfo};
use crate::core::runtime::detect_runtime_features;

use crate::chat::models::*;

const DEFAULT_BASE_URL: &str = "https://chat.googleapis.com/v1/";
const PROBE_PATH: &str = "spaces";

macro_rules! execute_json {
    ($client:ident, $($request:tt)*) => {{
        async {
            let response = $client.core.execute($($request)*).await?;
            let value = response.json().await?;
            Ok::<_, GrrError>(value)
        }
    }};
}

macro_rules! execute_delete {
    ($client:ident, $($request:tt)*) => {{
        async {
            $client.core.execute($($request)*).await.map(|_| ())
        }
    }};
}

pub struct ChatClientBuilder {
    auth: Option<GoogleAuth>,
    base_url: Option<Url>,
}

impl ChatClientBuilder {
    pub fn new() -> Self {
        Self {
            auth: None,
            base_url: None,
        }
    }

    pub fn auth(mut self, auth: GoogleAuth) -> Self {
        self.auth = Some(auth);
        self
    }

    pub fn base_url(mut self, url: Url) -> Self {
        self.base_url = Some(url);
        self
    }

    pub async fn build(self) -> Result<ChatClient> {
        let auth = self
            .auth
            .ok_or_else(|| GrrError::Config("Auth is required".into()))?;
        let has_base_override = self.base_url.is_some();
        let base_url = match self.base_url {
            Some(url) => url,
            None => Url::parse(DEFAULT_BASE_URL)
                .map_err(|e| GrrError::Config(format!("Invalid base URL: {e}")))?,
        };

        let core = if has_base_override {
            HttpCore::unprobed(auth, crate::core::http::build_http_client())
        } else {
            HttpCore::connect(auth, &base_url, PROBE_PATH).await
        };
        ChatClient::new(core, base_url)
    }
}

impl Default for ChatClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct ChatClient {
    core: HttpCore,
    base_url: Url,
}

impl ChatClient {
    pub fn new(core: HttpCore, base_url: Url) -> Result<Self> {
        let features = detect_runtime_features();
        info!(
            "ChatClient initialized: http3={}, io_uring={}",
            features.http3, features.io_uring
        );
        Ok(Self { core, base_url })
    }

    pub fn core(&self) -> &HttpCore {
        &self.core
    }

    pub fn transport_info(&self) -> &TransportInfo {
        self.core.transport_info()
    }

    pub fn token_backend(&self) -> &'static str {
        self.core.auth().token_backend()
    }

    pub async fn login(&self) -> Result<TokenStorage> {
        self.core.auth().login().await
    }

    pub async fn request_device_code(&self) -> Result<DeviceAuthChallenge> {
        self.core.auth().request_device_code().await
    }

    pub async fn poll_device_code(
        &self,
        challenge: &mut DeviceAuthChallenge,
    ) -> Result<Option<TokenStorage>> {
        self.core.auth().poll_device_code(challenge).await
    }

    pub async fn access_token(&self) -> Result<String> {
        self.core.auth().get_access_token().await
    }

    fn api_url(&self, path: &str) -> Result<Url> {
        self.base_url
            .join(path)
            .map_err(|e| GrrError::Config(format!("Invalid API URL: {e}")))
    }

    fn space_path(&self, space_id: &str) -> String {
        let bare = space_id.strip_prefix("spaces/").unwrap_or(space_id);
        format!("spaces/{}", encode_segment(bare))
    }

    fn space_url(&self, space_id: &str) -> Result<Url> {
        self.api_url(&self.space_path(space_id))
    }

    fn messages_url(&self, space_id: &str) -> Result<Url> {
        self.api_url(&format!("{}/messages", self.space_path(space_id)))
    }

    fn message_path(&self, space_id: &str, message_id: &str) -> String {
        if let Some(message) = message_id.strip_prefix("messages/") {
            format!(
                "{}/messages/{}",
                self.space_path(space_id),
                encode_segment(message)
            )
        } else if message_id.contains('/') {
            encode_resource_name(message_id)
        } else {
            format!(
                "{}/messages/{}",
                self.space_path(space_id),
                encode_segment(message_id)
            )
        }
    }

    fn messages_reactions_url(&self, space_id: &str, message_id: &str) -> Result<Url> {
        self.api_url(&format!(
            "{}/reactions",
            self.message_path(space_id, message_id)
        ))
    }

    fn memberships_url(&self, space_id: &str) -> Result<Url> {
        self.api_url(&format!("{}/members", self.space_path(space_id)))
    }

    fn membership_url(&self, space_id: &str, membership_id: &str) -> Result<Url> {
        let path = if let Some(member) = membership_id
            .strip_prefix("memberships/")
            .or_else(|| membership_id.strip_prefix("members/"))
        {
            format!(
                "{}/members/{}",
                self.space_path(space_id),
                encode_resource_name(member)
            )
        } else if membership_id.starts_with("spaces/") {
            normalize_membership_resource_name(membership_id)
        } else {
            format!(
                "{}/members/{}",
                self.space_path(space_id),
                encode_resource_name(membership_id)
            )
        };
        self.api_url(&path)
    }

    fn reaction_url(&self, space_id: &str, reaction_name: &str) -> Result<Url> {
        let path = if let Some(reaction) = reaction_name.strip_prefix("reactions/") {
            format!(
                "{}/reactions/{}",
                self.space_path(space_id),
                encode_segment(reaction)
            )
        } else if reaction_name.contains('/') {
            encode_resource_name(reaction_name)
        } else {
            format!(
                "{}/reactions/{}",
                self.space_path(space_id),
                encode_segment(reaction_name)
            )
        };
        self.api_url(&path)
    }

    pub async fn list_spaces(&self, max: Option<usize>) -> Result<Vec<Space>> {
        let batch_size = max.map_or(1000, |m| m.min(1000));

        paginate(max, move |page_token| async move {
            let mut request = self
                .core
                .get(self.api_url("spaces")?)
                .query(&[("pageSize", &batch_size.to_string())]);

            if let Some(token) = page_token.as_deref() {
                request = request.query(&[("pageToken", token)]);
            }

            let page: ListSpacesResponse = execute_json!(self, request).await?;
            Ok((page.spaces.unwrap_or_default(), page.next_page_token))
        })
        .await
    }

    pub async fn get_space(&self, space_id: &str) -> Result<Space> {
        execute_json!(self, self.core.get(self.space_url(space_id)?)).await
    }

    pub async fn list_messages(&self, space_id: &str, max: Option<usize>) -> Result<Vec<Message>> {
        let batch_size = max.map_or(1000, |m| m.min(1000));

        paginate(max, move |page_token| async move {
            let mut request = self
                .core
                .get(self.messages_url(space_id)?)
                .query(&[("pageSize", &batch_size.to_string())]);

            if let Some(token) = page_token.as_deref() {
                request = request.query(&[("pageToken", token)]);
            }

            let page: ListMessagesResponse = execute_json!(self, request).await?;
            Ok((page.messages.unwrap_or_default(), page.next_page_token))
        })
        .await
    }

    pub async fn send_message(&self, space_id: &str, text: &str) -> Result<Message> {
        let request = CreateMessageRequest {
            text: text.to_string(),
        };
        execute_json!(
            self,
            self.core.post(self.messages_url(space_id)?).json(&request)
        )
        .await
    }

    pub async fn setup_space<M>(
        &self,
        display_name: &str,
        description: Option<&str>,
        members: M,
    ) -> Result<Space>
    where
        M: IntoIterator,
        M::Item: AsRef<str>,
    {
        self.setup_space_with_read_mask(display_name, description, members, None)
            .await
    }

    pub async fn setup_space_with_read_mask<M>(
        &self,
        display_name: &str,
        description: Option<&str>,
        members: M,
        read_mask: Option<&str>,
    ) -> Result<Space>
    where
        M: IntoIterator,
        M::Item: AsRef<str>,
    {
        if display_name.trim().is_empty() {
            return Err(GrrError::InvalidArgument(
                "display name must not be empty".into(),
            ));
        }

        let memberships = members
            .into_iter()
            .map(|member| Membership {
                member: Some(MembershipMember {
                    name: Some(member.as_ref().to_string()),
                    ..MembershipMember::default()
                }),
                ..Membership::default()
            })
            .collect();
        let request = SetupSpaceRequest {
            space: SetupSpace {
                space_type: "SPACE".into(),
                display_name: display_name.to_string(),
                description: description.map(str::to_string),
                single_user_bot_dm: None,
            },
            memberships,
        };
        let mut builder = self
            .core
            .post(self.api_url("./spaces:setup")?)
            .json(&request);
        if let Some(mask) = read_mask {
            builder = builder.query(&[("readMask", mask)]);
        }
        execute_json!(self, builder).await
    }

    pub async fn patch_space(
        &self,
        space_id: &str,
        display_name: Option<&str>,
        description: Option<&str>,
    ) -> Result<Space> {
        if display_name.is_none() && description.is_none() {
            return Err(GrrError::InvalidArgument(
                "at least one space field must be supplied".into(),
            ));
        }

        let mut mask = Vec::new();
        if display_name.is_some() {
            mask.push("displayName");
        }
        if description.is_some() {
            mask.push("description");
        }
        let request = PatchSpaceRequest {
            display_name: display_name.map(str::to_string),
            description: description.map(str::to_string),
        };
        let url = self.space_url(space_id)?;
        match execute_json!(
            self,
            self.core
                .patch(url.clone())
                .query(&[("updateMask", &mask.join(","))])
                .json(&request),
        )
        .await
        {
            Ok(space) => Ok(space),
            Err(GrrError::NotFound(_)) => {
                let official_request = OfficialPatchSpaceRequest {
                    name: self.space_path(space_id),
                    display_name: display_name.map(str::to_string),
                    space_details: description.map(|value| SpaceDetails {
                        description: Some(value.to_string()),
                        guidelines: None,
                    }),
                };
                let mut official_mask = Vec::new();
                if display_name.is_some() {
                    official_mask.push("displayName");
                }
                if description.is_some() {
                    official_mask.push("spaceDetails");
                }
                let official_mask = official_mask.join(",");
                execute_json!(
                    self,
                    self.core
                        .patch(self.space_url(space_id)?)
                        .query(&[("updateMask", official_mask.as_str())])
                        .json(&official_request),
                )
                .await
            }
            Err(error) => Err(error),
        }
    }

    pub async fn update_space(
        &self,
        space_id: &str,
        display_name: Option<&str>,
        description: Option<&str>,
    ) -> Result<Space> {
        self.patch_space(space_id, display_name, description).await
    }

    pub async fn delete_space(&self, space_id: &str) -> Result<()> {
        execute_delete!(self, self.core.delete(self.space_url(space_id)?)).await
    }

    pub async fn list_memberships(
        &self,
        space_id: &str,
        max: Option<usize>,
    ) -> Result<Vec<Membership>> {
        let batch_size = max.map_or(1000, |value| value.min(1000));
        let url = self.memberships_url(space_id)?;

        paginate(max, move |page_token| {
            let url = url.clone();
            async move {
                let mut request = self
                    .core
                    .get(url)
                    .query(&[("pageSize", &batch_size.to_string())]);

                if let Some(token) = page_token.as_deref() {
                    request = request.query(&[("pageToken", token)]);
                }

                let page: ListMembershipsResponse = execute_json!(self, request).await?;
                Ok((page.memberships.unwrap_or_default(), page.next_page_token))
            }
        })
        .await
    }

    pub async fn get_membership(&self, space_id: &str, membership_id: &str) -> Result<Membership> {
        execute_json!(
            self,
            self.core.get(self.membership_url(space_id, membership_id)?)
        )
        .await
    }

    pub async fn create_membership(&self, space_id: &str, member_name: &str) -> Result<Membership> {
        if member_name.trim().is_empty() {
            return Err(GrrError::InvalidArgument(
                "member name must not be empty".into(),
            ));
        }
        let request = CreateMembershipRequest {
            member: MembershipMember {
                name: Some(member_name.to_string()),
                ..MembershipMember::default()
            },
        };
        execute_json!(
            self,
            self.core
                .post(self.memberships_url(space_id)?)
                .json(&request),
        )
        .await
    }

    pub async fn delete_membership(&self, space_id: &str, membership_id: &str) -> Result<()> {
        execute_delete!(
            self,
            self.core
                .delete(self.membership_url(space_id, membership_id)?),
        )
        .await
    }

    pub async fn list_reactions(
        &self,
        space_id: &str,
        message_id: &str,
        max: Option<usize>,
    ) -> Result<Vec<Reaction>> {
        let batch_size = max.map_or(200, |m| m.min(200));
        let url = self.messages_reactions_url(space_id, message_id)?;

        paginate(max, move |page_token| {
            let url = url.clone();
            async move {
                let mut request = self
                    .core
                    .get(url.clone())
                    .query(&[("pageSize", &batch_size.to_string())]);

                if let Some(token) = page_token.as_deref() {
                    request = request.query(&[("pageToken", token)]);
                }

                let page: ListReactionsResponse = execute_json!(self, request).await?;
                Ok((page.reactions.unwrap_or_default(), page.next_page_token))
            }
        })
        .await
    }

    pub async fn create_reaction(
        &self,
        space_id: &str,
        message_id: &str,
        emoji: &str,
    ) -> Result<Reaction> {
        let unicode = emoji_unicode(emoji);
        if unicode.is_empty() {
            return Err(GrrError::InvalidArgument("emoji must not be empty".into()));
        }
        let request = CreateReactionRequest {
            emoji: Emoji {
                unicode: Some(unicode),
                custom_emoji: None,
            },
        };
        execute_json!(
            self,
            self.core
                .post(self.messages_reactions_url(space_id, message_id)?)
                .json(&request),
        )
        .await
    }

    pub async fn create_reaction_with_emoji(
        &self,
        space_id: &str,
        message_id: &str,
        emoji: Emoji,
    ) -> Result<Reaction> {
        let request = CreateReactionRequest { emoji };
        execute_json!(
            self,
            self.core
                .post(self.messages_reactions_url(space_id, message_id)?)
                .json(&request),
        )
        .await
    }

    pub async fn delete_reaction(&self, space_id: &str, reaction_name: &str) -> Result<()> {
        execute_delete!(
            self,
            self.core
                .delete(self.reaction_url(space_id, reaction_name)?),
        )
        .await
    }
}

fn encode_segment(value: &str) -> String {
    urlencoding::encode(value).into_owned()
}

fn encode_resource_name(value: &str) -> String {
    value
        .split('/')
        .map(encode_segment)
        .collect::<Vec<_>>()
        .join("/")
}

fn normalize_membership_resource_name(value: &str) -> String {
    encode_resource_name(value).replacen("/memberships/", "/members/", 1)
}

fn emoji_unicode(value: &str) -> String {
    let value = value.trim();
    let candidate = value
        .strip_prefix("U+")
        .or_else(|| value.strip_prefix("u+"))
        .unwrap_or(value);
    if !candidate.is_empty()
        && candidate.len() <= 8
        && candidate
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        return candidate.to_ascii_uppercase();
    }

    candidate
        .chars()
        .map(|character| format!("{:X}", character as u32))
        .collect::<Vec<_>>()
        .join("-")
}

async fn paginate<T, F, Fut>(max: Option<usize>, mut fetch: F) -> Result<Vec<T>>
where
    F: FnMut(Option<String>) -> Fut,
    Fut: Future<Output = Result<(Vec<T>, Option<String>)>>,
{
    let mut all_items = Vec::new();
    let mut page_token: Option<String> = None;

    loop {
        let (items, next_page_token) = fetch(page_token.clone()).await?;
        all_items.extend(items);
        if let Some(maximum) = max
            && all_items.len() >= maximum
        {
            all_items.truncate(maximum);
            break;
        }

        page_token = next_page_token;
        if page_token.is_none() {
            break;
        }
    }

    Ok(all_items)
}

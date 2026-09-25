//! High-performance Google Calendar client: typed endpoints over [`HttpCore`].

use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tracing::info;
use url::Url;

use crate::core::auth::{DeviceAuthChallenge, GoogleAuth, TokenStorage};
use crate::core::error::{GrrError, Result};
use crate::core::http::{HttpCore, QueryParams, TransportInfo, join_url, parse_url};
use crate::core::runtime::detect_runtime_features;
use crate::core::{Page, paginate};

use crate::calendar::models::*;

const DEFAULT_BASE_URL: &str = "https://www.googleapis.com/calendar/v3/";
const PROBE_PATH: &str = "users/me/calendarList";

pub struct CalendarClientBuilder {
    auth: Option<GoogleAuth>,
    base_url: Option<Url>,
}

impl CalendarClientBuilder {
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

    pub async fn build(self) -> Result<CalendarClient> {
        let auth = self
            .auth
            .ok_or_else(|| GrrError::Config("Auth is required".into()))?;
        let has_base_override = self.base_url.is_some();
        let base_url = match self.base_url {
            Some(url) => url,
            None => parse_url(DEFAULT_BASE_URL, "base")?,
        };

        let core = if has_base_override {
            HttpCore::unprobed(auth, crate::core::http::build_http_client()?)
        } else {
            HttpCore::connect(auth, &base_url, PROBE_PATH).await?
        };
        CalendarClient::new(core, base_url).await
    }
}

impl Default for CalendarClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct CalendarClient {
    core: HttpCore,
    base_url: Url,
}

impl CalendarClient {
    pub async fn new(core: HttpCore, base_url: Url) -> Result<Self> {
        let features = detect_runtime_features().await;
        info!(
            "CalendarClient initialized: http3=always, io_uring={}",
            features.io_uring
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
        join_url(&self.base_url, path, "API")
    }

    fn calendar_url(&self, calendar_id: &str) -> Result<Url> {
        self.api_url(&format!("calendars/{}", urlencoding::encode(calendar_id)))
    }

    fn calendar_list_entry_url(&self, calendar_id: &str) -> Result<Url> {
        self.api_url(&format!(
            "users/me/calendarList/{}",
            urlencoding::encode(calendar_id)
        ))
    }

    fn events_url(&self, calendar_id: &str) -> Result<Url> {
        self.api_url(&format!(
            "calendars/{}/events",
            urlencoding::encode(calendar_id)
        ))
    }

    fn event_url(&self, calendar_id: &str, event_id: &str) -> Result<Url> {
        self.api_url(&format!(
            "calendars/{}/events/{}",
            urlencoding::encode(calendar_id),
            urlencoding::encode(event_id)
        ))
    }

    fn event_instances_url(&self, calendar_id: &str, event_id: &str) -> Result<Url> {
        self.api_url(&format!(
            "calendars/{}/events/{}/instances",
            urlencoding::encode(calendar_id),
            urlencoding::encode(event_id)
        ))
    }

    fn event_move_url(&self, calendar_id: &str, event_id: &str) -> Result<Url> {
        self.api_url(&format!(
            "calendars/{}/events/{}/move",
            urlencoding::encode(calendar_id),
            urlencoding::encode(event_id)
        ))
    }

    fn event_watch_url(&self, calendar_id: &str) -> Result<Url> {
        self.api_url(&format!(
            "calendars/{}/events/watch",
            urlencoding::encode(calendar_id)
        ))
    }

    fn acl_url(&self, calendar_id: &str) -> Result<Url> {
        self.api_url(&format!(
            "calendars/{}/acl",
            urlencoding::encode(calendar_id)
        ))
    }

    fn acl_rule_url(&self, calendar_id: &str, rule_id: &str) -> Result<Url> {
        self.api_url(&format!(
            "calendars/{}/acl/{}",
            urlencoding::encode(calendar_id),
            urlencoding::encode(rule_id)
        ))
    }

    pub async fn list_calendars(&self, max: Option<usize>) -> Result<Vec<CalendarListEntry>> {
        let batch_size = max.map_or(250, |value| value.min(250));

        paginate(max, move |page_token| async move {
            let query = QueryParams::new()
                .add("maxResults", batch_size.to_string())
                .add_page_token(page_token.as_deref());
            let page: CalendarList = self
                .core
                .execute_json(query.apply(self.core.get(self.api_url("users/me/calendarList")?)))
                .await?;
            Ok(Page::new(page.items, page.next_page_token))
        })
        .await
    }

    pub async fn get_calendar(&self, calendar_id: &str) -> Result<Calendar> {
        let response = self
            .core
            .execute(self.core.get(self.calendar_url(calendar_id)?))
            .await?;
        Ok(response.json().await?)
    }

    pub async fn create_calendar<T>(&self, calendar: T) -> Result<Calendar>
    where
        T: Serialize,
    {
        let response = self
            .core
            .execute(self.core.post(self.api_url("calendars")?).json(&calendar))
            .await?;
        Ok(response.json().await?)
    }

    pub async fn insert_calendar<T>(&self, calendar: T) -> Result<Calendar>
    where
        T: Serialize,
    {
        self.create_calendar(calendar).await
    }

    pub async fn update_calendar<T>(&self, calendar_id: &str, patch: T) -> Result<Calendar>
    where
        T: Serialize,
    {
        let response = self
            .core
            .execute(
                self.core
                    .patch(self.calendar_url(calendar_id)?)
                    .json(&patch),
            )
            .await?;
        Ok(response.json().await?)
    }

    pub async fn patch_calendar<T>(&self, calendar_id: &str, patch: T) -> Result<Calendar>
    where
        T: Serialize,
    {
        self.update_calendar(calendar_id, patch).await
    }

    pub async fn delete_calendar(&self, calendar_id: &str) -> Result<()> {
        self.core
            .execute(self.core.delete(self.calendar_url(calendar_id)?))
            .await?;
        Ok(())
    }

    pub async fn patch_calendar_list_entry<T>(
        &self,
        calendar_id: &str,
        patch: T,
    ) -> Result<CalendarListEntry>
    where
        T: Serialize,
    {
        let response = self
            .core
            .execute(
                self.core
                    .patch(self.calendar_list_entry_url(calendar_id)?)
                    .json(&patch),
            )
            .await?;
        Ok(response.json().await?)
    }

    pub async fn update_calendar_list_entry<T>(
        &self,
        calendar_id: &str,
        patch: T,
    ) -> Result<CalendarListEntry>
    where
        T: Serialize,
    {
        self.patch_calendar_list_entry(calendar_id, patch).await
    }

    pub async fn set_calendar_list_entry<T>(
        &self,
        calendar_id: &str,
        patch: T,
    ) -> Result<CalendarListEntry>
    where
        T: Serialize,
    {
        self.patch_calendar_list_entry(calendar_id, patch).await
    }

    pub async fn patch_calendar_list<T>(
        &self,
        calendar_id: &str,
        patch: T,
    ) -> Result<CalendarListEntry>
    where
        T: Serialize,
    {
        self.patch_calendar_list_entry(calendar_id, patch).await
    }

    pub async fn list_events(
        &self,
        calendar_id: &str,
        opts: EventListOptions,
    ) -> Result<Vec<Event>> {
        let batch_size = opts.max_results.map_or(2500, |value| value.min(2500));
        let time_min = opts.time_min.as_deref();
        let time_max = opts.time_max.as_deref();
        let query = opts.query.as_deref();

        paginate(opts.max_results, move |page_token| async move {
            let params = QueryParams::new()
                .add("singleEvents", "true")
                .add("maxResults", batch_size.to_string())
                .add_optional("timeMin", time_min)
                .add_optional("timeMax", time_max)
                .add_optional("q", query)
                .add_page_token(page_token.as_deref());
            let page: Events = self
                .core
                .execute_json(params.apply(self.core.get(self.events_url(calendar_id)?)))
                .await?;
            Ok(Page::new(page.items, page.next_page_token))
        })
        .await
    }

    pub async fn get_event(&self, calendar_id: &str, event_id: &str) -> Result<Event> {
        let response = self
            .core
            .execute(self.core.get(self.event_url(calendar_id, event_id)?))
            .await?;
        Ok(response.json().await?)
    }

    pub async fn create_event(&self, calendar_id: &str, event: Event) -> Result<Event> {
        let response = self
            .core
            .execute(self.core.post(self.events_url(calendar_id)?).json(&event))
            .await?;
        Ok(response.json().await?)
    }

    pub async fn update_event(
        &self,
        calendar_id: &str,
        event_id: &str,
        event: Event,
    ) -> Result<Event> {
        let response = self
            .core
            .execute(
                self.core
                    .put(self.event_url(calendar_id, event_id)?)
                    .json(&event),
            )
            .await?;
        Ok(response.json().await?)
    }

    pub async fn patch_event<T>(&self, calendar_id: &str, event_id: &str, patch: T) -> Result<Event>
    where
        T: Serialize,
    {
        let response = self
            .core
            .execute(
                self.core
                    .patch(self.event_url(calendar_id, event_id)?)
                    .json(&patch),
            )
            .await?;
        Ok(response.json().await?)
    }

    pub async fn delete_event(&self, calendar_id: &str, event_id: &str) -> Result<()> {
        self.core
            .execute(self.core.delete(self.event_url(calendar_id, event_id)?))
            .await?;
        Ok(())
    }

    pub async fn list_event_instances(
        &self,
        calendar_id: &str,
        event_id: &str,
        opts: EventInstancesOptions,
    ) -> Result<Vec<Event>> {
        let batch_size = opts.max_results.map_or(2500, |value| value.min(2500));
        let time_min = opts.time_min.as_deref();
        let time_max = opts.time_max.as_deref();

        paginate(opts.max_results, move |page_token| async move {
            let params = QueryParams::new()
                .add("maxResults", batch_size.to_string())
                .add_optional("timeMin", time_min)
                .add_optional("timeMax", time_max)
                .add_page_token(page_token.as_deref());
            let page: Events = self
                .core
                .execute_json(
                    params.apply(
                        self.core
                            .get(self.event_instances_url(calendar_id, event_id)?),
                    ),
                )
                .await?;
            Ok(Page::new(page.items, page.next_page_token))
        })
        .await
    }

    pub async fn get_event_instances(
        &self,
        calendar_id: &str,
        event_id: &str,
        opts: EventInstancesOptions,
    ) -> Result<Vec<Event>> {
        self.list_event_instances(calendar_id, event_id, opts).await
    }

    pub async fn event_instances(
        &self,
        calendar_id: &str,
        event_id: &str,
        max: Option<usize>,
        time_min: Option<&str>,
        time_max: Option<&str>,
    ) -> Result<Vec<Event>> {
        self.list_event_instances(
            calendar_id,
            event_id,
            EventInstancesOptions {
                max_results: max,
                time_min: time_min.map(str::to_owned),
                time_max: time_max.map(str::to_owned),
            },
        )
        .await
    }

    pub async fn list_instances(
        &self,
        calendar_id: &str,
        event_id: &str,
        max: Option<usize>,
        time_min: Option<&str>,
        time_max: Option<&str>,
    ) -> Result<Vec<Event>> {
        self.event_instances(calendar_id, event_id, max, time_min, time_max)
            .await
    }

    pub async fn move_event(
        &self,
        calendar_id: &str,
        event_id: &str,
        destination: &str,
    ) -> Result<Event> {
        let request = self
            .core
            .post(self.event_move_url(calendar_id, event_id)?)
            .query(&[("destination", destination)]);
        let response = self.core.execute(request).await?;
        Ok(response.json().await?)
    }

    pub async fn move_event_to(
        &self,
        calendar_id: &str,
        event_id: &str,
        destination: &str,
    ) -> Result<Event> {
        self.move_event(calendar_id, event_id, destination).await
    }

    pub async fn watch_events<T>(&self, calendar_id: &str, channel: T) -> Result<Channel>
    where
        T: Serialize,
    {
        let response = self
            .core
            .execute(
                self.core
                    .post(self.event_watch_url(calendar_id)?)
                    .json(&channel),
            )
            .await?;
        Ok(response.json().await?)
    }

    pub async fn watch_calendar(&self, calendar_id: &str, webhook: &str) -> Result<Channel> {
        let channel = Channel::webhook(generate_channel_id(), webhook);
        self.watch_events(calendar_id, &channel).await
    }

    pub async fn stop_channel<T>(&self, channel: T) -> Result<()>
    where
        T: Serialize,
    {
        self.core
            .execute(
                self.core
                    .post(self.api_url("channels/stop")?)
                    .json(&channel),
            )
            .await?;
        Ok(())
    }

    pub async fn stop_watch(&self, channel_id: &str, resource_id: &str) -> Result<()> {
        self.stop_channel(Channel::stop(channel_id, resource_id))
            .await
    }

    pub async fn get_colors(&self) -> Result<Colors> {
        let response = self
            .core
            .execute(self.core.get(self.api_url("colors")?))
            .await?;
        Ok(response.json().await?)
    }

    pub async fn colors(&self) -> Result<Colors> {
        self.get_colors().await
    }

    pub async fn list_settings(&self) -> Result<Settings> {
        let pages = paginate(None, |page_token| async move {
            let params = QueryParams::new().add_page_token(page_token.as_deref());
            let page: Settings = self
                .core
                .execute_json(params.apply(self.core.get(self.api_url("users/me/settings")?)))
                .await?;
            let next_page_token = page.next_page_token.clone();
            Ok(Page::new(vec![page], next_page_token))
        })
        .await?;

        let mut settings = Settings::default();
        for page in pages {
            if settings.kind.is_none() {
                settings.kind = page.kind;
            }
            if settings.etag.is_none() {
                settings.etag = page.etag;
            }
            settings.items.extend(page.items);
            settings.extra.extend(page.extra);
            settings.next_sync_token = page.next_sync_token;
        }
        Ok(settings)
    }

    pub async fn settings(&self) -> Result<Settings> {
        self.list_settings().await
    }

    pub async fn list_acl(&self, calendar_id: &str, max: Option<usize>) -> Result<Vec<AclRule>> {
        let batch_size = max.map_or(250, |value| value.min(250));

        paginate(max, move |page_token| async move {
            let params = QueryParams::new()
                .add("maxResults", batch_size.to_string())
                .add_page_token(page_token.as_deref());
            let page: AclList = self
                .core
                .execute_json(params.apply(self.core.get(self.acl_url(calendar_id)?)))
                .await?;
            Ok(Page::new(page.items, page.next_page_token))
        })
        .await
    }

    pub async fn list_acl_rules(
        &self,
        calendar_id: &str,
        max: Option<usize>,
    ) -> Result<Vec<AclRule>> {
        self.list_acl(calendar_id, max).await
    }

    pub async fn get_acl_rule(&self, calendar_id: &str, rule_id: &str) -> Result<AclRule> {
        let response = self
            .core
            .execute(self.core.get(self.acl_rule_url(calendar_id, rule_id)?))
            .await?;
        Ok(response.json().await?)
    }

    pub async fn get_acl(&self, calendar_id: &str, rule_id: &str) -> Result<AclRule> {
        self.get_acl_rule(calendar_id, rule_id).await
    }

    pub async fn insert_acl_rule<T>(
        &self,
        calendar_id: &str,
        rule: T,
        send_notifications: Option<bool>,
    ) -> Result<AclRule>
    where
        T: Serialize,
    {
        let mut request = self.core.post(self.acl_url(calendar_id)?).json(&rule);
        if let Some(send_notifications) = send_notifications {
            request = request.query(&[("sendNotifications", &send_notifications.to_string())]);
        }
        let response = self.core.execute(request).await?;
        Ok(response.json().await?)
    }

    pub async fn insert_acl<T>(
        &self,
        calendar_id: &str,
        rule: T,
        send_notifications: Option<bool>,
    ) -> Result<AclRule>
    where
        T: Serialize,
    {
        self.insert_acl_rule(calendar_id, rule, send_notifications)
            .await
    }

    pub async fn patch_acl_rule<T>(
        &self,
        calendar_id: &str,
        rule_id: &str,
        patch: T,
        send_notifications: Option<bool>,
    ) -> Result<AclRule>
    where
        T: Serialize,
    {
        let mut request = self
            .core
            .patch(self.acl_rule_url(calendar_id, rule_id)?)
            .json(&patch);
        if let Some(send_notifications) = send_notifications {
            request = request.query(&[("sendNotifications", &send_notifications.to_string())]);
        }
        let response = self.core.execute(request).await?;
        Ok(response.json().await?)
    }

    pub async fn update_acl_rule<T>(
        &self,
        calendar_id: &str,
        rule_id: &str,
        patch: T,
        send_notifications: Option<bool>,
    ) -> Result<AclRule>
    where
        T: Serialize,
    {
        self.patch_acl_rule(calendar_id, rule_id, patch, send_notifications)
            .await
    }

    pub async fn patch_acl<T>(
        &self,
        calendar_id: &str,
        rule_id: &str,
        patch: T,
        send_notifications: Option<bool>,
    ) -> Result<AclRule>
    where
        T: Serialize,
    {
        self.patch_acl_rule(calendar_id, rule_id, patch, send_notifications)
            .await
    }

    pub async fn delete_acl_rule(&self, calendar_id: &str, rule_id: &str) -> Result<()> {
        self.core
            .execute(self.core.delete(self.acl_rule_url(calendar_id, rule_id)?))
            .await?;
        Ok(())
    }

    pub async fn delete_acl(&self, calendar_id: &str, rule_id: &str) -> Result<()> {
        self.delete_acl_rule(calendar_id, rule_id).await
    }

    pub async fn freebusy(
        &self,
        calendar_ids: &[&str],
        time_min: &str,
        time_max: &str,
    ) -> Result<FreeBusyResponse> {
        let request = FreeBusyRequest {
            time_min: time_min.to_string(),
            time_max: time_max.to_string(),
            items: calendar_ids
                .iter()
                .map(|id| FreeBusyItem { id: id.to_string() })
                .collect(),
        };

        let response = self
            .core
            .execute(self.core.post(self.api_url("freeBusy")?).json(&request))
            .await?;
        Ok(response.json().await?)
    }
}

fn generate_channel_id() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("grr-{}-{timestamp:x}", std::process::id())
}

//! High-performance Google Calendar client: typed endpoints over [`HttpCore`].
//!
//! Transport, retry, rate-valving, and OAuth live in grr-core; this crate
//! adds Calendar's URL space, models, and list pagination.

use tracing::info;
use url::Url;

use grr_core::auth::{DeviceAuthChallenge, GoogleAuth, TokenStorage};
use grr_core::error::{GrrError, Result};
use grr_core::http::{HttpCore, TransportInfo};
use grr_core::runtime::detect_runtime_features;

use crate::models::*;

const DEFAULT_BASE_URL: &str = "https://www.googleapis.com/calendar/v3/";
/// Any authenticated request proves the transport; calendarList is the
/// cheapest authenticated GET in Calendar's URL space.
const PROBE_PATH: &str = "users/me/calendarList";

/// Google Calendar API client builder.
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

    /// Override the Calendar API base URL (primarily for test injection).
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
            None => Url::parse(DEFAULT_BASE_URL)
                .map_err(|e| GrrError::Config(format!("Invalid base URL: {}", e)))?,
        };

        let core = if has_base_override {
            // Explicit base URL (test injection): skip the transport probe —
            // mock servers speak HTTP/1.1 and QUIC packets would just time out.
            HttpCore::unprobed(auth, grr_core::http::build_http_client())
        } else {
            HttpCore::connect(auth, &base_url, PROBE_PATH).await
        };
        CalendarClient::new(core, base_url)
    }
}

impl Default for CalendarClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// High-performance Google Calendar client.
#[derive(Clone)]
pub struct CalendarClient {
    core: HttpCore,
    base_url: Url,
}

impl CalendarClient {
    /// Create new client from a connected core (test injection point).
    pub fn new(core: HttpCore, base_url: Url) -> Result<Self> {
        let features = detect_runtime_features();
        info!(
            "CalendarClient initialized: http3={}, io_uring={}",
            features.http3, features.io_uring
        );
        Ok(Self { core, base_url })
    }

    /// The shared HTTP engine (advanced use; typed methods preferred).
    pub fn core(&self) -> &HttpCore {
        &self.core
    }

    /// Transport negotiation details observed during client construction.
    pub fn transport_info(&self) -> &TransportInfo {
        self.core.transport_info()
    }

    /// Which token backend is live ("os-keyring" or "file").
    pub fn token_backend(&self) -> &'static str {
        self.core.auth().token_backend()
    }

    /// Fresh interactive login (PKCE browser flow), dropping any stored
    /// token first so a dead credential can never block consent.
    pub async fn login(&self) -> Result<TokenStorage> {
        self.core.auth().login().await
    }

    /// Start an OAuth device flow; display the challenge to the user.
    pub async fn request_device_code(&self) -> Result<DeviceAuthChallenge> {
        self.core.auth().request_device_code().await
    }

    /// Poll a device-flow challenge. `Ok(None)` means keep waiting.
    pub async fn poll_device_code(
        &self,
        challenge: &mut DeviceAuthChallenge,
    ) -> Result<Option<TokenStorage>> {
        self.core.auth().poll_device_code(challenge).await
    }

    /// Get access token
    pub async fn access_token(&self) -> Result<String> {
        self.core.auth().get_access_token().await
    }

    /// Build API URL with proper error handling
    fn api_url(&self, path: &str) -> Result<Url> {
        self.base_url
            .join(path)
            .map_err(|e| GrrError::Config(format!("Invalid API URL: {}", e)))
    }

    /// URL of a calendar's events collection (calendar IDs are path
    /// segments and may contain '@'/'.'/'#' — always percent-encode).
    fn events_url(&self, calendar_id: &str) -> Result<Url> {
        self.api_url(&format!(
            "calendars/{}/events",
            urlencoding::encode(calendar_id)
        ))
    }

    /// URL of a single event resource
    fn event_url(&self, calendar_id: &str, event_id: &str) -> Result<Url> {
        self.api_url(&format!(
            "calendars/{}/events/{}",
            urlencoding::encode(calendar_id),
            urlencoding::encode(event_id)
        ))
    }

    // ══════════════════════════════════════════════════════════════════
    // Calendar Operations
    // ══════════════════════════════════════════════════════════════════

    /// List the user's calendars (calendarList) with automatic pagination
    /// up to `max` entries (all pages when `None`).
    pub async fn list_calendars(&self, max: Option<usize>) -> Result<Vec<CalendarListEntry>> {
        let mut all_calendars = Vec::new();
        let mut page_token: Option<String> = None;
        // calendarList.list caps a single page at 250 entries.
        let batch_size = max.map_or(250, |m| m.min(250));

        loop {
            let mut request = self
                .core
                .get(self.api_url("users/me/calendarList")?)
                .query(&[("maxResults", &batch_size.to_string())]);

            if let Some(token) = &page_token {
                request = request.query(&[("pageToken", token)]);
            }

            let response = self.core.execute(request).await?;
            let page: CalendarList = response.json().await?;

            all_calendars.extend(page.items);
            if let Some(m) = max
                && all_calendars.len() >= m
            {
                all_calendars.truncate(m);
                break;
            }

            page_token = page.next_page_token;
            if page_token.is_none() {
                break;
            }
        }

        Ok(all_calendars)
    }

    /// List events on a calendar with automatic pagination up to
    /// `opts.max_results` events (all pages when `None`).
    ///
    /// `singleEvents=true` expands recurring events into their instances,
    /// which is the shape a listing wants (IDs are per-instance and work
    /// with get/update/delete).
    pub async fn list_events(
        &self,
        calendar_id: &str,
        opts: EventListOptions,
    ) -> Result<Vec<Event>> {
        let mut all_events = Vec::new();
        let mut page_token: Option<String> = None;
        // events.list caps a single page at 2,500 entries.
        let batch_size = opts.max_results.map_or(2500, |m| m.min(2500));

        loop {
            let mut request = self.core.get(self.events_url(calendar_id)?).query(&[
                ("singleEvents", "true"),
                ("maxResults", &batch_size.to_string()),
            ]);

            if let Some(time_min) = &opts.time_min {
                request = request.query(&[("timeMin", time_min)]);
            }
            if let Some(time_max) = &opts.time_max {
                request = request.query(&[("timeMax", time_max)]);
            }
            if let Some(query) = &opts.query {
                request = request.query(&[("q", query)]);
            }
            if let Some(token) = &page_token {
                request = request.query(&[("pageToken", token)]);
            }

            let response = self.core.execute(request).await?;
            let page: Events = response.json().await?;

            all_events.extend(page.items);
            if let Some(m) = opts.max_results
                && all_events.len() >= m
            {
                all_events.truncate(m);
                break;
            }

            page_token = page.next_page_token;
            if page_token.is_none() {
                break;
            }
        }

        Ok(all_events)
    }

    /// Get a single event by ID
    pub async fn get_event(&self, calendar_id: &str, event_id: &str) -> Result<Event> {
        let response = self
            .core
            .execute(self.core.get(self.event_url(calendar_id, event_id)?))
            .await?;
        Ok(response.json().await?)
    }

    /// Create an event (POST a full Event body; the CLI builds it from
    /// its flags)
    pub async fn create_event(&self, calendar_id: &str, event: Event) -> Result<Event> {
        let response = self
            .core
            .execute(self.core.post(self.events_url(calendar_id)?).json(&event))
            .await?;
        Ok(response.json().await?)
    }

    /// Update (PUT) an event with a full Event body
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

    /// Delete an event (204 No Content on success)
    pub async fn delete_event(&self, calendar_id: &str, event_id: &str) -> Result<()> {
        self.core
            .execute(self.core.delete(self.event_url(calendar_id, event_id)?))
            .await?;
        Ok(())
    }

    // ══════════════════════════════════════════════════════════════════
    // Free/Busy
    // ══════════════════════════════════════════════════════════════════

    /// Query free/busy periods for a set of calendars over a time window
    /// (`time_min`/`time_max` are RFC3339 timestamps).
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

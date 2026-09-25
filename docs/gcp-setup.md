# Google Cloud setup for grr

One-time setup, about 5 minutes. You need three things: the Google Cloud CLI, a Google Cloud project with the Gmail API enabled, and a Desktop OAuth client ID.

Shortcut: [scripts/setup-gcp.ps1](../scripts/setup-gcp.ps1) (Windows) and [scripts/setup-gcp.sh](../scripts/setup-gcp.sh) (macOS/Linux) automate sign-in, project selection, and enabling the Gmail API, then print the console links for steps 5–6.

## 1. Install the Google Cloud CLI

Windows (PowerShell):

```powershell
winget install Google.CloudSDK
```

macOS:

```sh
brew install --cask google-cloud-sdk
```

Linux (apt/dnf/etc.): follow the official instructions at <https://cloud.google.com/sdk/docs/install>.

## 2. Sign in to Google

```sh
gcloud auth login
```

## 3. Pick (or create) a project

List what you have and select one:

```sh
gcloud projects list
gcloud config set project YOUR_PROJECT_ID
```

Starting from scratch:

```sh
gcloud projects create YOUR_PROJECT_ID
gcloud config set project YOUR_PROJECT_ID
```

A personal project on the free tier comfortably covers Gmail API usage for your own mailbox.

## 4. Enable the APIs

Gmail is required; enable the rest as you need them (Calendar, Drive, Contacts, Chat, Forms are all live `grr` services):

```sh
gcloud services enable gmail.googleapis.com

# all the other grr services at once:
gcloud services enable calendar-json.googleapis.com drive.googleapis.com people.googleapis.com chat.googleapis.com forms.googleapis.com
```

The setup scripts enable only `gmail.googleapis.com`; run the command above manually to enable the rest.

Note: Google Keep's API is Workspace-enterprise-only (no consumer API), so Keep will never appear as a `grr` service.

## 5. OAuth consent screen

Open <https://console.cloud.google.com/apis/credentials/consent>:

1. User type: **External**
2. Fill in the minimal form. Use **Rust Rewrite** as the app name, choose a support email, and upload the orange crab logo from [`assets/logo.svg`](../assets/logo.svg). The project/fork is still **Google Rust Rewrite**; **Rust Rewrite** is the name shown by Google's consent screen.
3. Use the project's public URLs when the form asks for them:
   - Homepage: <https://grr-cli.pages.dev/>
   - Privacy policy: <https://grr-cli.pages.dev/privacy/>
   - Terms: <https://grr-cli.pages.dev/terms/>
4. Add `grr-cli.pages.dev` as the authorized domain.
5. Add the Google account you will sign in with as a **Test user**.

One login covers every `grr` service, so the consent screen will ask for all of them — Gmail (read/compose/modify/labels), Calendar, Drive, Contacts, Chat (messages/spaces/memberships/reactions), and Forms (body/responses) — even if you only plan to use one. Enable the matching APIs (step 4) for the services you use.

## 6. Create the OAuth client ID

Open <https://console.cloud.google.com/apis/credentials>:

1. **Create credentials → OAuth client ID**
2. Application type: **Desktop app** (name it anything, e.g. `grr`)
3. Copy the **client ID** (ends in `.apps.googleusercontent.com`) and the **client secret** shown next to it

Put both in `~/.grr/config.toml` (Windows: `%USERPROFILE%\.grr\config.toml`):

```toml
[oauth]
client_id = "123456789-abc.apps.googleusercontent.com"
client_secret = "GOCSPX-..."
```

Google shows a client secret even for Desktop clients. `grr` sends it at the token endpoint only when present — PKCE is always on either way. To set the secret without echoing it into shell history, use [scripts/set-client-secret.ps1](../scripts/set-client-secret.ps1) (hidden prompt).

## 7. Log in with grr

```sh
grr auth login
```

Your browser opens, you consent, done. On a headless machine use `grr auth login --device` and follow the printed URL + code. Verify with:

```sh
grr auth status
grr gmail profile
```

While the consent screen is in **Testing** mode, Google expires refresh tokens after about 7 days — rerun `grr auth login` when that happens. The 0.4 release also adds `chat.delete`, `chat.memberships`, `chat.messages.reactions`, and `contacts.other.readonly`; existing users must run `grr auth login` again to grant those scopes. Publishing the app avoids the testing-mode expiry, but is unnecessary for personal use.

## Agent environments: no MCP setup

`grr` replaces per-service MCP servers with one fast CLI. There is no MCP setup for `grr` itself: point your agent at the `grr` binary and have it read `grr schema` for the complete machine-readable command contract.

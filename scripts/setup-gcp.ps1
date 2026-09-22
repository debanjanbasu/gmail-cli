[CmdletBinding()]
param(
  [string]$ProjectId
)

$ErrorActionPreference = 'Stop'

if (-not (Get-Command gcloud -ErrorAction SilentlyContinue)) {
  Write-Host "gcloud not found; installing Google Cloud CLI via winget"
  winget install Google.CloudSDK
  if (-not (Get-Command gcloud -ErrorAction SilentlyContinue)) {
    throw "gcloud installed but not on PATH yet; open a new PowerShell window and rerun this script"
  }
}

gcloud auth login

if (-not $ProjectId) {
  Write-Host "Projects:"
  gcloud projects list --format="table(projectId,name)"
  $ProjectId = Read-Host -Prompt 'Project ID to use (empty to create a new one)'
  if (-not $ProjectId) {
    $ProjectId = Read-Host -Prompt 'ID for the new project'
    gcloud projects create $ProjectId
  }
}

gcloud config set project $ProjectId
gcloud services enable gmail.googleapis.com

Write-Host ""
Write-Host "gcloud setup complete. Finish in the browser:"
Write-Host "  1. OAuth consent screen: https://console.cloud.google.com/apis/credentials/consent"
Write-Host "     Choose External, fill the minimal form, and add your own Google account email as a Test user."
Write-Host "  2. Create the OAuth client ID: https://console.cloud.google.com/apis/credentials"
Write-Host "     Create credentials -> OAuth client ID -> Application type: Desktop"
Write-Host "  3. Put the client ID (ends .apps.googleusercontent.com) and secret into ~/.grr/config.toml"
Write-Host "  4. Run: grr auth login"

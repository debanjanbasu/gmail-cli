#!/usr/bin/env bash
set -euo pipefail

PROJECT_ID="${1:-}"

if ! command -v gcloud >/dev/null 2>&1; then
  echo "gcloud not found."
  echo "  macOS: brew install --cask google-cloud-sdk"
  echo "  Linux: https://cloud.google.com/sdk/docs/install"
  exit 1
fi

gcloud auth login

if [ -z "$PROJECT_ID" ]; then
  echo "Projects:"
  gcloud projects list
  read -r -p "Project ID to use (empty to create a new one): " PROJECT_ID
  if [ -z "$PROJECT_ID" ]; then
    read -r -p "ID for the new project: " PROJECT_ID
    gcloud projects create "$PROJECT_ID"
  fi
fi

gcloud config set project "$PROJECT_ID"
gcloud services enable gmail.googleapis.com

echo
echo "gcloud setup complete. Finish in the browser:"
echo "  1. OAuth consent screen: https://console.cloud.google.com/apis/credentials/consent"
echo "     Choose External, fill the minimal form, and add your own Google account email as a Test user."
echo "  2. Create the OAuth client ID: https://console.cloud.google.com/apis/credentials"
echo "     Create credentials -> OAuth client ID -> Application type: Desktop"
echo "  3. Put the client ID (ends .apps.googleusercontent.com) and secret into ~/.grr/config.toml"
echo "  4. Run: grr auth login"

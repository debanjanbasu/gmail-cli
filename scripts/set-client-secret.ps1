<#
.SYNOPSIS
  Sets the Gmail OAuth client_secret in config.toml without echoing it.

.DESCRIPTION
  Updates (or adds) the `client_secret` line under `[oauth]` in the Gmail
  config file. The secret never appears on screen, in transcripts, or in
  command history — it lives in this process memory only, and the BSTR is
  zeroed after use.

  Only needed for providers that mandate a secret at the token endpoint
  (Google does, even for Desktop clients). PKCE-only providers omit it
  entirely — the binary sends no secret when the line is absent.

  Three modes (first match wins):
    1. Interactive (default)  – hidden prompt via Read-Host -AsSecureString.
    2. -EnvVarName <name>     – read plaintext from that env var (CI/vault
                                agents that inject secrets as env vars).
    3. -Secret <SecureString> – caller-supplied SecureString, e.g.
                                $s = Read-Host -AsSecureString
                                .\set-client-secret.ps1 -Secret $s

  Programmatic alternative (no file edit at all): GMAIL_OAUTH__CLIENT_SECRET
  env var takes priority over config.toml (see ConfigLoader).

.EXAMPLE
  .\set-client-secret.ps1
  # hidden prompt, updates ~/.gmail-opencode/config.toml

.EXAMPLE
  .\set-client-secret.ps1 -EnvVarName MY_VAULT_SECRET
  # CI: secret comes from the $env:MY_VAULT_SECRET variable

.EXAMPLE
  $s = ConvertTo-SecureString (op read 'op://vault/gmail/client-secret') -AsPlainText -Force
  .\set-client-secret.ps1 -Secret $s
  # 1Password CLI, secret never typed
#>
[CmdletBinding(DefaultParameterSetName = 'Interactive')]
param(
  [Parameter()]
  [string]$ConfigPath = $(
    if ($env:GMAIL_CONFIG_PATH) { $env:GMAIL_CONFIG_PATH }
    else { Join-Path $env:USERPROFILE '.gmail-opencode\config.toml' }
  ),

  [Parameter(ParameterSetName = 'FromEnv', Mandatory = $true)]
  [string]$EnvVarName,

  [Parameter(ParameterSetName = 'FromSecure', Mandatory = $true)]
  [SecureString]$Secret
)

$ErrorActionPreference = 'Stop'

if (-not (Test-Path -LiteralPath $ConfigPath)) {
  throw "config not found: $ConfigPath"
}

$secure = $null
$fromEnvPlain = $null
switch ($PSCmdlet.ParameterSetName) {
  'FromEnv' {
    $fromEnvPlain = [Environment]::GetEnvironmentVariable($EnvVarName)
    if ([string]::IsNullOrWhiteSpace($fromEnvPlain)) {
      throw "env var $EnvVarName is empty or missing"
    }
    $secure = ConvertTo-SecureString $fromEnvPlain -AsPlainText -Force
  }
  'FromSecure' { $secure = $Secret }
  default {
    $secure = Read-Host -Prompt 'Paste OAuth client secret (input hidden)' -AsSecureString
  }
}

$bstr = [IntPtr]::Zero
try {
  $bstr = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($secure)
  $plain = [Runtime.InteropServices.Marshal]::PtrToStringBSTR($bstr)
  if ([string]::IsNullOrWhiteSpace($plain)) { throw 'empty secret, aborting' }
  if ($plain.Length -lt 10) { Write-Warning 'secret looks unusually short, continuing anyway' }

  $text = Get-Content -LiteralPath $ConfigPath -Raw
  $escaped = ($plain -replace '\\', '\\') -replace '"', '\"'
  # MatchEvaluator result is literal: safe even if the secret contains '$'.
  $pattern = '(?m)^client_secret\s*=.*$'
  $replacement = { param($m) "client_secret = `"$escaped`"" }.GetNewClosure()
  if ($text -match $pattern) {
    $text = [regex]::Replace($text, $pattern, $replacement, 1)
  } else {
    $text = $text.TrimEnd() + "`nclient_secret = `"$escaped`"`n"
  }

  Set-Content -LiteralPath $ConfigPath -Value $text
  "client_secret updated in $ConfigPath"
  'Next: gmail auth login'
} finally {
  if ($bstr -ne [IntPtr]::Zero) {
    [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($bstr)
  }
  foreach ($v in @('plain', 'secure', 'fromEnvPlain')) {
    Remove-Variable -Name $v -ErrorAction SilentlyContinue
  }
}

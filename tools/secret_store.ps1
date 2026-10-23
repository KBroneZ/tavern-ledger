# The hosted Supabase project's URL and secret key for the owner's scripts
# (T-570; dot-sourced by backup_supabase.ps1, sim_consented_games.ps1,
# supabase_secret.ps1 and the tests).
#
# Store: one JSON file outside the repo, by default
# %LOCALAPPDATA%\tavern-ledger-dev\supabase-secret.json, in a folder only the
# current user may open:
#   { "format": "tl-supabase-secret-1", "url": "https://<project>.supabase.co",
#     "key_dpapi": "<base64 of the key encrypted with DPAPI CurrentUser>" }
# The URL is not a secret (the desktop app has it built in); the key is only
# ever decrypted into memory.
#
# What DPAPI CurrentUser protects: the key at rest (backups, disk images, other
# Windows users, an upload or `cat` of the repo folder). What it does not: any
# process running as the same Windows user, AI sessions included, can decrypt it.

Set-StrictMode -Version Latest
Add-Type -AssemblyName System.Security.Cryptography.ProtectedData

$script:SecretStoreFormat = 'tl-supabase-secret-1'
# Ties the blob to this use; not a secret.
$script:SecretStoreEntropy = [Text.Encoding]::UTF8.GetBytes('tavern-ledger supabase secret key v1')
$script:SecretEnvNames = @('SUPABASE_URL', 'SUPABASE_SECRET_KEY')
$script:SecretCheckTimeoutSec = 15

function Get-DefaultSecretStorePath {
    return Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) 'tavern-ledger-dev\supabase-secret.json'
}

function Assert-SupabaseUrl([string]$Url) {
    if ($Url -notmatch '^https://[a-z0-9]+\.supabase\.co$') {
        throw 'SUPABASE_URL must be https://<project>.supabase.co'
    }
}

# Messages never quote the key.
function Assert-SupabaseSecretKey([string]$Key) {
    if ([string]::IsNullOrEmpty($Key) -or -not $Key.StartsWith('sb_secret_')) {
        throw 'SUPABASE_SECRET_KEY must be a secret key (sb_secret_...), not a legacy service_role key.'
    }
    if ($Key -cnotmatch '^sb_secret_[A-Za-z0-9_-]+$') {
        throw 'SUPABASE_SECRET_KEY has unexpected characters.'
    }
}

# Reads the old plaintext file as data (never sourced or run). Anything but
# blank lines and one SUPABASE_URL and one SUPABASE_SECRET_KEY line stops it,
# naming only the variable or the line number, so nothing in the file is lost
# when it is deleted.
function Read-SupabaseEnvFile([string]$Path) {
    $vars = @{}
    $n = 0
    foreach ($line in [IO.File]::ReadAllLines($Path)) {
        $n++
        if ($line -match '^\s*$') { continue }
        if ($line -notmatch '^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=(.*)$') {
            throw "Line $n of the env file is not NAME=value; stopping."
        }
        $name = $Matches[1]
        $value = $Matches[2].Trim()
        if ($name -cnotin $script:SecretEnvNames) {
            throw "The env file holds $name, which has no place in the store; stopping."
        }
        if ($vars.ContainsKey($name)) { throw "The env file sets $name twice; stopping." }
        if ($value.Length -ge 2 -and $value.StartsWith('"') -and $value.EndsWith('"')) {
            $value = $value.Substring(1, $value.Length - 2)
        }
        $vars[$name] = $value
    }
    foreach ($name in $script:SecretEnvNames) {
        if (-not $vars[$name]) { throw "The env file has no $name." }
    }
    Assert-SupabaseUrl $vars['SUPABASE_URL']
    Assert-SupabaseSecretKey $vars['SUPABASE_SECRET_KEY']
    return @{ Url = $vars['SUPABASE_URL']; Key = $vars['SUPABASE_SECRET_KEY'] }
}

function Protect-SecretText([string]$Text) {
    $plain = [Text.Encoding]::UTF8.GetBytes($Text)
    try {
        $blob = [Security.Cryptography.ProtectedData]::Protect($plain, $script:SecretStoreEntropy, 'CurrentUser')
        return [Convert]::ToBase64String($blob)
    }
    finally { [Array]::Clear($plain) }
}

function Unprotect-SecretText([string]$Base64) {
    $plain = [Security.Cryptography.ProtectedData]::Unprotect(
        [Convert]::FromBase64String($Base64), $script:SecretStoreEntropy, 'CurrentUser')
    try { return [Text.Encoding]::UTF8.GetString($plain) }
    finally { [Array]::Clear($plain) }
}

function ConvertFrom-SecureText([securestring]$Secure) {
    $ptr = [Runtime.InteropServices.Marshal]::SecureStringToGlobalAllocUnicode($Secure)
    try { return [Runtime.InteropServices.Marshal]::PtrToStringUni($ptr) }
    finally { [Runtime.InteropServices.Marshal]::ZeroFreeGlobalAllocUnicode($ptr) }
}

function Test-SameConnection([hashtable]$A, [hashtable]$B) {
    return $A.Url -ceq $B.Url -and $A.Key -ceq $B.Key
}

# Decrypts the stored key into memory. No store is an error: the scripts never
# fall back to the plaintext file.
function Read-SecretStore([string]$Path = (Get-DefaultSecretStorePath)) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "No stored Supabase key at $Path. Run pwsh tools\supabase_secret.ps1 to move .local\supabase.env there."
    }
    try { $doc = [IO.File]::ReadAllText($Path) | ConvertFrom-Json -AsHashtable }
    catch { throw "The key store at $Path is not valid JSON; fix or delete it by hand." }
    if ($doc -isnot [hashtable] -or $doc['format'] -cne $script:SecretStoreFormat -or
        $doc['url'] -isnot [string] -or $doc['key_dpapi'] -isnot [string]) {
        throw "The key store at $Path is not a $($script:SecretStoreFormat) file; fix or delete it by hand."
    }
    try { $key = Unprotect-SecretText $doc['key_dpapi'] }
    catch { throw "The stored key could not be decrypted (another Windows user, or a broken file at $Path)." }
    Assert-SupabaseUrl $doc['url']
    Assert-SupabaseSecretKey $key
    return @{ Url = $doc['url']; Key = $key }
}

# Writes the store atomically: a temp file in the same folder, then a rename,
# so an interrupted run leaves the old store or none, never half a file.
# Without -Replace an existing store is never overwritten.
function Write-SecretStore([hashtable]$Connection, [string]$Path, [switch]$Replace) {
    Assert-SupabaseUrl $Connection.Url
    Assert-SupabaseSecretKey $Connection.Key
    $dir = Split-Path -Parent $Path
    $leaf = [IO.Path]::GetFileNameWithoutExtension($Path)
    New-Item -ItemType Directory -Force $dir | Out-Null
    & icacls $dir /inheritance:r /grant:r "$($env:USERNAME):(OI)(CI)F" | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "Could not restrict access to $dir." }
    # Left by a run that was cut off; they hold only encrypted bytes.
    Get-ChildItem -LiteralPath $dir -File -Filter "$leaf.*.partial" |
        ForEach-Object { Remove-Item -LiteralPath $_.FullName -Force -Confirm:$false }

    $json = [ordered]@{
        format    = $script:SecretStoreFormat
        url       = $Connection.Url
        key_dpapi = Protect-SecretText $Connection.Key
    } | ConvertTo-Json
    $tmp = Join-Path $dir "$leaf.$([guid]::NewGuid().ToString('N')).partial"
    try {
        [IO.File]::WriteAllText($tmp, $json)
        [IO.File]::Move($tmp, $Path, [bool]$Replace)
    }
    finally {
        if (Test-Path -LiteralPath $tmp) { Remove-Item -LiteralPath $tmp -Force -Confirm:$false }
    }
}

# Moves a connection into the store once. Returns 'stored' or 'already stored'
# (the same URL and key are there); a different stored one is never overwritten.
function Import-SupabaseSecret([hashtable]$Connection, [string]$Path) {
    if (Test-Path -LiteralPath $Path) {
        if (Test-SameConnection (Read-SecretStore $Path) $Connection) { return 'already stored' }
        throw "A different Supabase URL or key is already stored at $Path; not overwriting it. After a key rotation use -SetKey."
    }
    Write-SecretStore $Connection $Path
    if (-not (Test-SameConnection (Read-SecretStore $Path) $Connection)) {
        Remove-Item -LiteralPath $Path -Force -Confirm:$false
        throw 'The stored key did not read back the same; the store was removed.'
    }
    return 'stored'
}

# What the owner's scripts use: the local stack's keys with -Local, otherwise
# the stored hosted project's.
function Get-SupabaseConnection([switch]$Local, [string]$StorePath = (Get-DefaultSecretStorePath)) {
    if ($Local) {
        $raw = (& npx supabase status -o json 2>$null | Out-String)
        if ($LASTEXITCODE -ne 0) { throw 'Local stack is not running (npx supabase start).' }
        $status = ($raw -replace '(?s)^[^{]*', '') | ConvertFrom-Json
        if (-not $status.SECRET_KEY) { throw 'supabase status gave no SECRET_KEY (update the Supabase CLI).' }
        return @{ Url = $status.API_URL; Key = $status.SECRET_KEY }
    }
    return Read-SecretStore $StorePath
}

# A harmless authenticated read: GET <url>/storage/v1/bucket/games with the
# key in apikey only (never as a bearer token). Only a secret key may read a
# private bucket's settings; the reply has its name and settings, no user data,
# and nothing is written. Expected: HTTP 200, the private `games` bucket.
function Test-SupabaseSecretWorks([hashtable]$Connection) {
    try {
        $res = Invoke-WebRequest -Method Get -Uri "$($Connection.Url)/storage/v1/bucket/games" `
            -Headers @{ apikey = $Connection.Key } -TimeoutSec $script:SecretCheckTimeoutSec -SkipHttpErrorCheck
    }
    catch {
        throw "The check request failed: $($_.Exception.Message.Replace($Connection.Key, '<key>'))"
    }
    if ($res.StatusCode -ne 200) {
        throw "The check got HTTP $($res.StatusCode), not 200: the key is not accepted."
    }
    try { $bucket = $res.Content | ConvertFrom-Json -AsHashtable }
    catch { throw 'The check reply is not JSON.' }
    if ($bucket -isnot [hashtable] -or $bucket['name'] -cne 'games' -or
        $bucket['public'] -isnot [bool] -or $bucket['public']) {
        throw 'The check reply is not the private games bucket.'
    }
}

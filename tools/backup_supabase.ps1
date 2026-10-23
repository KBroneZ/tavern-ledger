<#
.SYNOPSIS
Weekly backup of the Supabase project until it moves to Pro (D-020, T-104a).
Writes one encrypted file; no plaintext is kept on disk.

.DESCRIPTION
Runs on the developer's own machine, never in CI (the repo is public).
Each run builds, in a temp folder outside the repo, a folder with
  roles.sql, schema.sql, data.sql   supabase db dump
  storage\games\<path>              every file of the private `games` bucket
  manifest.json                     file list with sizes and SHA-256
zips it and encrypts the zip into <OutDir>\tl-backups-<UTC time>.tlenc
(TLENC1: PBKDF2-SHA256 600 000 iterations + AES-256-GCM). Open one with
decrypt-backups.ps1 from the same folder. The temp folder is deleted when the
run ends, also when it fails; a failed run leaves no file in OutDir.
Backups older than -KeepDays (default 35) are deleted, so a deleted account
leaves the backups within that time; the privacy policy (T-104b) states it.

Passphrase (at least 12 characters): read with Read-Host -AsSecureString and
asked twice, or taken from the environment variable TAVERN_BACKUP_PASSPHRASE
for scheduled runs. It is never written to disk, logged or put on a command
line. Keep it somewhere safe: without it a backup cannot be opened.

Hosted project (default): needs `npx supabase link` done once, and the
project's URL and secret key (sb_secret_...; legacy service_role keys are
refused, D-053) in the DPAPI store that tools\supabase_secret.ps1 fills.
-Local backs up the local stack instead (for testing the script).
Keys are only decrypted into memory, never printed or written to the backup.

.PARAMETER OutDir
Where the .tlenc files go. Default: Documents\TavernLedger-backups.

.PARAMETER KeepDays
Encrypted backups in OutDir older than this are deleted (7 to 365, default 35).

.EXAMPLE
pwsh tools\backup_supabase.ps1
pwsh tools\backup_supabase.ps1 -Local
pwsh tools\backup_supabase.ps1 -OutDir D:\backups
#>
#Requires -Version 7
[CmdletBinding()]
param(
    [switch]$Local,
    [string]$OutDir = (Join-Path ([Environment]::GetFolderPath('MyDocuments')) 'TavernLedger-backups'),
    [ValidateRange(7, 365)]
    [int]$KeepDays = 35
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'backup_crypto.ps1')
. (Join-Path $PSScriptRoot 'secret_store.ps1')

$repo = Split-Path -Parent $PSScriptRoot
$bucket = 'games'
$timeoutSec = 30
$pageSize = 1000

# The secret key goes in apikey only, never as a bearer token.
function Get-Headers($conn) {
    return @{ apikey = $conn.Key }
}

function Invoke-Dump([string]$file, [string[]]$extra) {
    $target = if ($Local) { '--local' } else { '--linked' }
    & npx supabase db dump $target @extra -f $file 2>&1 |
        Where-Object { $_ -notmatch 'new version|recommend updating|getting-started' } |
        ForEach-Object { Write-Host "  $_" }
    if ($LASTEXITCODE -ne 0) { throw "supabase db dump failed for $(Split-Path -Leaf $file)." }
}

# Every object name under a prefix. The Storage list call is not recursive:
# entries with a null id are folders.
function Get-ObjectNames($conn, [string]$prefix) {
    $names = [System.Collections.Generic.List[string]]::new()
    $offset = 0
    do {
        $body = @{ prefix = $prefix; limit = $pageSize; offset = $offset } | ConvertTo-Json
        # Invoke-RestMethod would hand the JSON array over as one object.
        $res = Invoke-WebRequest -Method Post -Uri "$($conn.Url)/storage/v1/object/list/$bucket" `
            -Headers (Get-Headers $conn) -ContentType 'application/json' -Body $body `
            -TimeoutSec $timeoutSec
        $page = @($res.Content | ConvertFrom-Json)
        foreach ($entry in $page) {
            $full = if ($prefix) { "$prefix/$($entry.name)" } else { $entry.name }
            if ($null -eq $entry.id) {
                foreach ($n in (Get-ObjectNames $conn $full)) { $names.Add($n) }
            }
            else { $names.Add($full) }
        }
        $offset += $page.Count
    } while ($page.Count -eq $pageSize)
    return $names
}

# Old backups go first, so a failing backup never keeps deleted users' data
# past -KeepDays.
Remove-ExpiredBackups -OutDir $OutDir -KeepDays $KeepDays

# Asked before anything is dumped, so a typo or a short passphrase costs nothing.
$passphrase = Read-BackupPassphrase
$conn = Get-SupabaseConnection -Local:$Local
$stamp = (Get-Date).ToUniversalTime().ToString('yyyyMMdd-HHmmss')
$outFile = Join-Path $OutDir "tl-backups-$stamp.tlenc"
Write-Host "Backup to $outFile"

Invoke-EncryptedBackup -OutFile $outFile -FolderName $stamp -Passphrase $passphrase -Work {
    param($dir)
    Write-Host 'Database...'
    Invoke-Dump (Join-Path $dir 'roles.sql') @('--role-only')
    Invoke-Dump (Join-Path $dir 'schema.sql') @()
    # Live sign-in secrets are left out: a restore signs everyone out, which is
    # safer than keeping usable refresh tokens on disk.
    Invoke-Dump (Join-Path $dir 'data.sql') @('--data-only', '--use-copy',
        '-x', 'auth.refresh_tokens', '-x', 'auth.sessions', '-x', 'auth.one_time_tokens',
        '-x', 'auth.flow_state')

    Write-Host "Bucket '$bucket'..."
    $storageDir = Join-Path $dir "storage\$bucket"
    $manifest = [System.Collections.Generic.List[object]]::new()
    foreach ($name in (Get-ObjectNames $conn '')) {
        # Names come from the server: refuse anything that could leave the folder.
        if ($name -notmatch '^[0-9a-f-]{36}/[A-Za-z0-9_.-]+$' -or $name.Contains('..')) {
            throw "Unexpected object name in the bucket; stopping."
        }
        $out = Join-Path $storageDir ($name -replace '/', '\')
        New-Item -ItemType Directory -Force (Split-Path -Parent $out) | Out-Null
        $escaped = ($name -split '/' | ForEach-Object { [uri]::EscapeDataString($_) }) -join '/'
        Invoke-WebRequest -Uri "$($conn.Url)/storage/v1/object/$bucket/$escaped" `
            -Headers (Get-Headers $conn) -OutFile $out -TimeoutSec $timeoutSec | Out-Null
        $manifest.Add([ordered]@{
            name   = $name
            bytes  = (Get-Item $out).Length
            sha256 = (Get-FileHash -Algorithm SHA256 $out).Hash.ToLowerInvariant()
        })
    }
    [ordered]@{
        created_utc = $stamp
        source      = if ($Local) { 'local' } else { 'linked' }
        files       = $manifest
    } | ConvertTo-Json -Depth 4 | Set-Content -Encoding utf8 (Join-Path $dir 'manifest.json')
    Write-Host "  $($manifest.Count) files"
}
Write-Host 'Done.'

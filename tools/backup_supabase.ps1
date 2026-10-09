<#
.SYNOPSIS
Weekly backup of the Supabase project until it moves to Pro (D-020, T-104a).
Keep .localackups on an encrypted disk (BitLocker): data.sql holds every
account's email and password hash.

.DESCRIPTION
Runs on the developer's own machine, never in CI (the repo is public).
Writes to .local\backups\<UTC time>\ :
  roles.sql, schema.sql, data.sql   supabase db dump
  storage\games\<path>              every file of the private `games` bucket
  manifest.json                     file list with sizes and SHA-256
Backups older than -KeepDays (default 35) are deleted, so a deleted account
leaves the backups within that time; the privacy policy (T-104b) states it.

Hosted project (default): needs `npx supabase link` done once, and
.local\supabase.env with SUPABASE_URL and SUPABASE_SERVICE_ROLE_KEY lines.
-Local backs up the local stack instead (for testing the script).
Keys are only read into memory, never printed or written to the backup.

.EXAMPLE
pwsh tools\backup_supabase.ps1
pwsh tools\backup_supabase.ps1 -Local
#>
#Requires -Version 7
[CmdletBinding()]
param(
    [switch]$Local,
    [ValidateRange(7, 365)]
    [int]$KeepDays = 35
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repo = Split-Path -Parent $PSScriptRoot
$backupRoot = Join-Path $repo '.local\backups'
$bucket = 'games'
$timeoutSec = 30
$pageSize = 1000

function Get-Connection {
    if ($Local) {
        $raw = (& npx supabase status -o json 2>$null | Out-String)
        if ($LASTEXITCODE -ne 0) { throw 'Local stack is not running (npx supabase start).' }
        $status = ($raw -replace '(?s)^[^{]*', '') | ConvertFrom-Json
        return @{ Url = $status.API_URL; Key = $status.SERVICE_ROLE_KEY }
    }
    $envFile = Join-Path $repo '.local\supabase.env'
    if (-not (Test-Path $envFile)) { throw "Missing $envFile (SUPABASE_URL, SUPABASE_SERVICE_ROLE_KEY)." }
    $vars = @{}
    foreach ($line in Get-Content $envFile) {
        if ($line -match '^\s*(SUPABASE_URL|SUPABASE_SERVICE_ROLE_KEY)\s*=\s*(.+?)\s*$') {
            $vars[$Matches[1]] = $Matches[2].Trim('"')
        }
    }
    if (-not $vars['SUPABASE_URL'] -or -not $vars['SUPABASE_SERVICE_ROLE_KEY']) {
        throw "$envFile needs SUPABASE_URL and SUPABASE_SERVICE_ROLE_KEY."
    }
    if ($vars['SUPABASE_URL'] -notmatch '^https://[a-z0-9]+\.supabase\.co$') {
        throw 'SUPABASE_URL must be https://<project>.supabase.co'
    }
    return @{ Url = $vars['SUPABASE_URL']; Key = $vars['SUPABASE_SERVICE_ROLE_KEY'] }
}

function Get-Headers($conn) {
    $h = @{ apikey = $conn.Key }
    if (-not $conn.Key.StartsWith('sb_')) { $h.Authorization = "Bearer $($conn.Key)" }
    return $h
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

function Remove-OldBackups {
    $cutoff = (Get-Date).ToUniversalTime().AddDays(-$KeepDays)
    Get-ChildItem $backupRoot -Directory -ErrorAction SilentlyContinue |
        Where-Object {
            $_.Name -match '^\d{8}-\d{6}$' -and
            [datetime]::ParseExact($_.Name, 'yyyyMMdd-HHmmss', $null) -lt $cutoff
        } |
        ForEach-Object {
            Write-Host "Deleting backup older than $KeepDays days: $($_.Name)"
            Remove-Item -Recurse -Force -Confirm:$false $_.FullName
        }
}

# Old backups go first, so a failing backup never keeps deleted users' data
# past -KeepDays.
Remove-OldBackups

$conn = Get-Connection
$stamp = (Get-Date).ToUniversalTime().ToString('yyyyMMdd-HHmmss')
$dir = Join-Path $backupRoot $stamp
New-Item -ItemType Directory -Force $dir | Out-Null
try {
    # Only the current user may read the backup (it holds every account's data).
    & icacls $dir /inheritance:r /grant:r "$($env:USERNAME):(OI)(CI)F" | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Could not restrict access to the backup folder.' }
    Write-Host "Backup to .local\backups\$stamp"

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
} catch {
    # A half-written backup is worse than none: it looks complete.
    Remove-Item -Recurse -Force -Confirm:$false $dir -ErrorAction SilentlyContinue
    throw
}
Write-Host 'Done.'

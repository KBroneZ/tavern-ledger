<#
.SYNOPSIS
Measures the combat simulator on the uploaded games of accounts that turned
on "Let my games be used to improve the simulator" (T-547, D-119).

.DESCRIPTION
Runs on the owner's own machine, never in CI. In one go:
  1. asks public.simulator_files() for the game files that may be used now
     (accounts whose switch is on at this moment, files still kept);
  2. downloads each into .local\sim-games\<UTC time>\ under a running number
     (0001.json, ...): the account id in the Storage path is never written to
     disk or printed, and the folder is readable by the current user only;
  3. checks each file against the SHA-256 the server stored for it and skips
     any that differ (replaced or broken), counting them;
  4. runs `combat-validate --games <folder> --cards <file> --summary`, which
     prints counts, shares, rules and card ids only;
  5. deletes the folder, whatever happened (the privacy policy promises the
     copies are deleted when the run ends).
Only counts are printed: never a path, account id or file name.

Hosted project (default): .local\supabase.env with SUPABASE_URL and
SUPABASE_SECRET_KEY lines, as for tools\backup_supabase.ps1. -Local uses the
local stack instead (for testing the script on synthetic data).
Keys are only read into memory, never printed or written.

.EXAMPLE
pwsh tools\sim_consented_games.ps1 -Cards $env:TEMP\hsjson\cards.json
pwsh tools\sim_consented_games.ps1 -Local -Cards crates\combat-sim\tests\data\cards.json
#>
#Requires -Version 7
[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [string]$Cards,
    [switch]$Local,
    [ValidateRange(1, 10000)]
    [int]$Limit = 1000
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repo = Split-Path -Parent $PSScriptRoot
$workRoot = Join-Path $repo '.local\sim-games'
$bucket = 'games'
$timeoutSec = 30

# As in tools\backup_supabase.ps1.
function Get-Connection {
    if ($Local) {
        $raw = (& npx supabase status -o json 2>$null | Out-String)
        if ($LASTEXITCODE -ne 0) { throw 'Local stack is not running (npx supabase start).' }
        $status = ($raw -replace '(?s)^[^{]*', '') | ConvertFrom-Json
        if (-not $status.SECRET_KEY) { throw 'supabase status gave no SECRET_KEY (update the Supabase CLI).' }
        return @{ Url = $status.API_URL; Key = $status.SECRET_KEY }
    }
    $envFile = Join-Path $repo '.local\supabase.env'
    if (-not (Test-Path $envFile)) { throw "Missing $envFile (SUPABASE_URL, SUPABASE_SECRET_KEY)." }
    $vars = @{}
    foreach ($line in Get-Content $envFile) {
        if ($line -match '^\s*(SUPABASE_URL|SUPABASE_SECRET_KEY)\s*=\s*(.+?)\s*$') {
            $vars[$Matches[1]] = $Matches[2].Trim('"')
        }
    }
    if (-not $vars['SUPABASE_URL'] -or -not $vars['SUPABASE_SECRET_KEY']) {
        throw "$envFile needs SUPABASE_URL and SUPABASE_SECRET_KEY."
    }
    if (-not $vars['SUPABASE_SECRET_KEY'].StartsWith('sb_secret_')) {
        throw 'SUPABASE_SECRET_KEY must be a secret key (sb_secret_...), not a legacy service_role key.'
    }
    if ($vars['SUPABASE_URL'] -notmatch '^https://[a-z0-9]+\.supabase\.co$') {
        throw 'SUPABASE_URL must be https://<project>.supabase.co'
    }
    return @{ Url = $vars['SUPABASE_URL']; Key = $vars['SUPABASE_SECRET_KEY'] }
}

# The secret key goes in apikey only, never as a bearer token.
function Get-Headers($conn) {
    return @{ apikey = $conn.Key }
}

# The files that may be used now: storage_path, content_sha256, size_bytes.
function Get-ConsentedFiles($conn) {
    $body = @{ p_limit = $Limit } | ConvertTo-Json
    # Invoke-RestMethod would hand the JSON array over as one object.
    $res = Invoke-WebRequest -Method Post -Uri "$($conn.Url)/rest/v1/rpc/simulator_files" `
        -Headers (Get-Headers $conn) -ContentType 'application/json' -Body $body `
        -TimeoutSec $timeoutSec
    return @($res.Content | ConvertFrom-Json)
}

# The inflated bytes of a gzipped file.
function Expand-Gzip([string]$path) {
    $in = [System.IO.File]::OpenRead($path)
    try {
        $gz = [System.IO.Compression.GZipStream]::new($in, [System.IO.Compression.CompressionMode]::Decompress)
        $out = [System.IO.MemoryStream]::new()
        $gz.CopyTo($out)
        return $out.ToArray()
    } finally {
        $in.Dispose()
    }
}

function Get-Sha256([byte[]]$bytes) {
    return [System.Convert]::ToHexString([System.Security.Cryptography.SHA256]::HashData($bytes)).ToLowerInvariant()
}

if (-not (Test-Path $Cards -PathType Leaf)) { throw "No card data at $Cards (HearthstoneJSON cards.json)." }

$conn = Get-Connection
$stamp = (Get-Date).ToUniversalTime().ToString('yyyyMMdd-HHmmss')
$dir = Join-Path $workRoot $stamp
New-Item -ItemType Directory -Force $dir | Out-Null
try {
    # Only the current user may read the copies.
    & icacls $dir /inheritance:r /grant:r "$($env:USERNAME):(OI)(CI)F" | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Could not restrict access to the work folder.' }

    # @() again: a function returning an empty array returns $null.
    $rows = @(Get-ConsentedFiles $conn)
    Write-Host "Games listed: $($rows.Count)"
    $kept = 0
    $skipped = 0
    $n = 0
    foreach ($row in $rows) {
        $n++
        # Names come from the server: refuse anything that could leave the folder.
        if ($row.storage_path -notmatch '^[0-9a-f-]{36}/Hearthstone_[0-9_]+-[0-9]+\.json\.gz$') {
            throw 'Unexpected object name in the list; stopping.'
        }
        $escaped = ($row.storage_path -split '/' | ForEach-Object { [uri]::EscapeDataString($_) }) -join '/'
        $gzFile = Join-Path $dir ('{0:D4}.json.gz' -f $n)
        Invoke-WebRequest -Uri "$($conn.Url)/storage/v1/object/$bucket/$escaped" `
            -Headers (Get-Headers $conn) -OutFile $gzFile -TimeoutSec $timeoutSec | Out-Null
        $bytes = Expand-Gzip $gzFile
        Remove-Item -Force $gzFile
        if ((Get-Sha256 $bytes) -ne $row.content_sha256) {
            $skipped++
            continue
        }
        [System.IO.File]::WriteAllBytes((Join-Path $dir ('{0:D4}.json' -f $n)), $bytes)
        $kept++
    }
    Write-Host "Games downloaded and checked: $kept (skipped, changed or broken: $skipped)"
    if ($kept -eq 0) {
        Write-Host 'Nothing to measure.'
        return
    }

    Push-Location $repo
    try {
        & cargo run --release --quiet -p combat-sim --bin combat-validate -- `
            --games $dir --cards (Resolve-Path $Cards).Path --summary
        if ($LASTEXITCODE -ne 0) { throw 'combat-validate failed.' }
    } finally {
        Pop-Location
    }
} finally {
    # The copies never outlive the run (privacy policy, "Improving the simulator").
    Remove-Item -Recurse -Force -Confirm:$false $dir -ErrorAction SilentlyContinue
    if (Test-Path $dir) { Write-Warning "Could not delete the work folder under .local\sim-games; delete it by hand." }
    else { Write-Host 'Copies deleted.' }
}

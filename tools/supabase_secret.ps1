<#
.SYNOPSIS
Keeps the hosted Supabase project's secret key encrypted at rest (T-570).

.DESCRIPTION
Runs on the owner's own machine, never in CI. The key is stored with Windows
DPAPI for the current user in %LOCALAPPDATA%\tavern-ledger-dev\supabase-secret.json
(see tools\secret_store.ps1), and tools\backup_supabase.ps1 and
tools\sim_consented_games.ps1 read it from there at run time. DPAPI protects
the key at rest (backups, disk images, other Windows users, an upload of the
repo folder); it does not stop a process running as the same Windows user,
AI sessions included, from decrypting it.

Default: moves SUPABASE_URL and SUPABASE_SECRET_KEY from .local\supabase.env
into the store (once; safe to re-run or interrupt, and a different stored key
is never overwritten), then checks the stored key the way the scripts read
it. The plaintext file is kept. Once the plaintext file is gone it only checks.

The check is one harmless authenticated read: GET
<SUPABASE_URL>/storage/v1/bucket/games with the key in apikey, 15 s timeout.
Expected: HTTP 200 with the private `games` bucket's settings. No user data
comes back and nothing is written.

The key is never printed, logged, put on a command line or in an
environment variable.

.PARAMETER DeletePlaintext
Does the same, then asks before deleting .local\supabase.env (refused if it
is a link). Its bytes may stay on the disk and in older backups of the PC.

.PARAMETER Check
Only decrypts the stored key in memory and runs the check.

.PARAMETER SetKey
After a key rotation: asks for the new secret key (hidden input), checks it,
then asks before replacing the stored one. The stored URL is kept.

.EXAMPLE
pwsh tools\supabase_secret.ps1
pwsh tools\supabase_secret.ps1 -DeletePlaintext
pwsh tools\supabase_secret.ps1 -Check
pwsh tools\supabase_secret.ps1 -SetKey
#>
#Requires -Version 7
[CmdletBinding(SupportsShouldProcess, ConfirmImpact = 'High', DefaultParameterSetName = 'Migrate')]
param(
    [Parameter(ParameterSetName = 'Migrate')]
    [switch]$DeletePlaintext,
    [Parameter(ParameterSetName = 'Check', Mandatory)]
    [switch]$Check,
    [Parameter(ParameterSetName = 'SetKey', Mandatory)]
    [switch]$SetKey,
    [Parameter(ParameterSetName = 'Migrate')]
    [string]$EnvFile = (Join-Path (Split-Path -Parent $PSScriptRoot) '.local\supabase.env'),
    [string]$StorePath
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'secret_store.ps1')
if ($WhatIfPreference) { throw '-WhatIf is not supported: the move writes the store.' }
if (-not $StorePath) { $StorePath = Get-DefaultSecretStorePath }
# .NET calls resolve relative paths against the process folder, not PowerShell's.
$StorePath = $PSCmdlet.GetUnresolvedProviderPathFromPSPath($StorePath)
$EnvFile = $PSCmdlet.GetUnresolvedProviderPathFromPSPath($EnvFile)

# Through the readers' own path, so a pass means they work.
function Invoke-Check {
    Test-SupabaseSecretWorks (Get-SupabaseConnection -StorePath $StorePath)
    Write-Host 'Check passed: the stored key reads the private games bucket (HTTP 200).'
}

if ($Check) {
    Invoke-Check
    return
}

if ($SetKey) {
    $stored = Read-SecretStore $StorePath
    $new = @{
        Url = $stored.Url
        Key = ConvertFrom-SecureText (Read-Host 'New Supabase secret key (sb_secret_...)' -AsSecureString)
    }
    Assert-SupabaseSecretKey $new.Key
    if (Test-SameConnection $stored $new) {
        Write-Host 'That key is already stored.'
        return
    }
    Test-SupabaseSecretWorks $new
    if ($PSCmdlet.ShouldProcess($StorePath, 'Replace the stored Supabase secret key')) {
        Write-SecretStore $new $StorePath -Replace
        Invoke-Check
    }
    return
}

if (-not (Test-Path -LiteralPath $EnvFile -PathType Leaf)) {
    if (-not (Test-Path -LiteralPath $StorePath)) { throw "No $EnvFile and no stored key at $StorePath." }
    Write-Host 'No plaintext file; the key is stored.'
    Invoke-Check
    return
}

$result = Import-SupabaseSecret (Read-SupabaseEnvFile $EnvFile) $StorePath
Write-Host "Key $result in $StorePath (DPAPI, user $($env:USERNAME))."
Invoke-Check

if (-not $DeletePlaintext) {
    Write-Host "Plaintext file kept: $EnvFile. Delete it with -DeletePlaintext."
    return
}
if (Test-IsLink $EnvFile) { throw "$EnvFile is a link; not deleting it. Remove it by hand." }
if ($PSCmdlet.ShouldProcess($EnvFile, 'Delete the plaintext Supabase key file')) {
    Remove-Item -LiteralPath $EnvFile -Force -Confirm:$false
    Write-Host "Deleted $EnvFile. Copies may remain on the disk and in older PC backups."
}

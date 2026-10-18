<#
.SYNOPSIS
  Builds the Microsoft Store edition of Tavern Ledger as an UNSIGNED .msix (T-379, D-158).

.DESCRIPTION
  1. Builds the app with the `store` cargo feature and tauri.store.conf.json (no installer,
     no updater) into its own target folder, so the installer edition's binary is never packed.
  2. Checks the exe is the Store edition with no updater in it (tools/store_package.py check-exe).
  3. Lays out the package folder (exe, logos, AppxManifest.xml) and runs `makeappx pack`.

  The package is NOT signed and nothing is uploaded or submitted: the Store signs it. The identity
  (name, publisher, publisher display name) comes from Partner Center once the account exists; until
  then the defaults are clearly fake (Tavern Ledger local test) and -ForSubmission refuses them.

.PARAMETER Variant
  full    the manifest excludes %LOCALAPPDATA%\Blizzard and %APPDATA%\TavernLedger from
          virtualization and asks for the unvirtualizedResources capability (Fix button works).
  manual  no restricted capability; builds the app with `store-manual-setup` (no Fix button).

.PARAMETER LocalTest
  A debug build with a launch alias (-TestAlias, default TLStoreTest.exe) so a test run can pass
  --data-dir and --local-app-data. Never for submission; keep it apart from the CI artifact.

.PARAMETER Exe
  Pack this exe instead of building one (it is still checked).

.EXAMPLE
  pwsh tools/make-msix.ps1                       # fake identity, full variant
  pwsh tools/make-msix.ps1 -ForSubmission -IdentityName X -Publisher "CN=..." -PublisherDisplayName Y
#>
[CmdletBinding()]
param(
  [ValidateSet('full', 'manual')][string]$Variant = 'full',
  [string]$IdentityName = $env:STORE_IDENTITY_NAME,
  [string]$Publisher = $env:STORE_PUBLISHER,
  [string]$PublisherDisplayName = $env:STORE_PUBLISHER_DISPLAY_NAME,
  [string]$OutDir = 'target\store-package',
  [string]$TargetDir = 'target\store',
  [switch]$ForSubmission,
  [switch]$LocalTest,
  [string]$TestAlias = 'TLStoreTest.exe',
  [string]$Exe
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

function Invoke-Native([string]$What, [scriptblock]$Run) {
  & $Run
  if ($LASTEXITCODE -ne 0) { throw "$What failed (exit $LASTEXITCODE)" }
}

if ($ForSubmission -and $LocalTest) { throw '-LocalTest and -ForSubmission cannot be combined.' }
$python = (Get-Command python -ErrorAction Stop).Source
# Relative to the repo, or absolute (a build folder on another drive).
$targetAbs = [System.IO.Path]::Combine($root, $TargetDir)
$outAbs = [System.IO.Path]::Combine($root, $OutDir)

# Identity: the Partner Center values, or the fake local-test ones (store_package.py says which).
$identityArgs = @()
if ($IdentityName) { $identityArgs += @('--name', $IdentityName) }
if ($Publisher) { $identityArgs += @('--publisher', $Publisher) }
if ($PublisherDisplayName) { $identityArgs += @('--publisher-display', $PublisherDisplayName) }
if ($ForSubmission) { $identityArgs += '--for-submission' }
if ($LocalTest) { $identityArgs += @('--test-alias', $TestAlias) }

$version = (& $python tools/store_version.py).Trim()
if ($LASTEXITCODE -ne 0) { throw 'no valid Store version for this app version' }
Write-Host "Store package version $version, variant $Variant"

# 1. Build (the installer edition's target folder is never used).
$profileDir = if ($LocalTest) { 'debug' } else { 'release' }
if (-not $Exe) {
  $features = if ($Variant -eq 'manual') { 'store,store-manual-setup' } else { 'store' }
  $env:CARGO_TARGET_DIR = $targetAbs
  Push-Location (Join-Path $root 'crates\desktop')
  try {
    $build = @('build', '--no-bundle', '--no-sign', '--config', 'tauri.store.conf.json', '--features', $features)
    if ($LocalTest) { $build += '--debug' }
    Invoke-Native 'tauri build' { & npx --no-install tauri @build -- --locked }
  }
  finally { Pop-Location }
  $Exe = Join-Path $env:CARGO_TARGET_DIR "$profileDir\TavernLedger.exe"
}
if (-not (Test-Path $Exe)) { throw "The exe is not there: $Exe" }

# 2. It must be the Store edition, with no updater in it.
Invoke-Native 'the exe check' { & $python tools/store_package.py check-exe $Exe }

# 3. The package folder, then the .msix.
$stage = Join-Path $outAbs 'folder'
Invoke-Native 'staging the package folder' {
  & $python tools/store_package.py stage --exe $Exe --out $stage --variant $Variant @identityArgs
}

$sdk = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin\*\x64\makeappx.exe" -ErrorAction SilentlyContinue |
  Sort-Object { [version]($_.Directory.Parent.Name) } -Descending | Select-Object -First 1
if (-not $sdk) { throw 'makeappx.exe not found: install the Windows 10/11 SDK.' }

$suffix = ''
if ($Variant -eq 'manual') { $suffix += '_manual' }
if ($LocalTest) { $suffix += '_localtest' }
$msix = Join-Path $outAbs "TavernLedger_${version}_x64$suffix.msix"
if (Test-Path $msix) { Remove-Item -Force $msix }
Invoke-Native 'makeappx pack' { & $sdk.FullName pack /v /d $stage /p $msix /o }

$hash = (Get-FileHash $msix -Algorithm SHA256).Hash.ToLowerInvariant()
Write-Host "Unsigned package: $msix"
Write-Host "SHA-256: $hash"
if ($env:GITHUB_STEP_SUMMARY) {
  "Store package ``$(Split-Path -Leaf $msix)`` (unsigned, not submitted). SHA-256 ``$hash``." |
    Add-Content -Path $env:GITHUB_STEP_SUMMARY
}

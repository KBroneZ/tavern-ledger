# Encryption helpers for tools\backup_supabase.ps1 (dot-sourced; also by the tests).
#
# File format TLENC1, the one decrypt-backups.ps1 opens:
#   "TLENC1" (6 bytes ASCII) | salt (16) | nonce (12) | GCM tag (16) | ciphertext
# Key: PBKDF2-HMAC-SHA256 of the UTF-8 passphrase, 600 000 iterations, 32 bytes.
# Cipher: AES-256-GCM, no associated data. The plaintext is a zip.

Set-StrictMode -Version Latest
Add-Type -AssemblyName System.IO.Compression.FileSystem

$script:TlencMagic = [Text.Encoding]::ASCII.GetBytes('TLENC1')
$script:TlencSaltBytes = 16
$script:TlencNonceBytes = 12
$script:TlencTagBytes = 16
$script:TlencIterations = 600000
$script:MinPassphraseLength = 12
$script:PassphraseEnvVar = 'TAVERN_BACKUP_PASSPHRASE'

function Get-PassphraseText([securestring]$secure) {
    $ptr = [Runtime.InteropServices.Marshal]::SecureStringToGlobalAllocUnicode($secure)
    try { return [Runtime.InteropServices.Marshal]::PtrToStringUni($ptr) }
    finally { [Runtime.InteropServices.Marshal]::ZeroFreeGlobalAllocUnicode($ptr) }
}

function Assert-PassphraseLength([string]$passphrase) {
    if ([string]::IsNullOrEmpty($passphrase) -or $passphrase.Length -lt $script:MinPassphraseLength) {
        throw "The passphrase must have at least $($script:MinPassphraseLength) characters."
    }
}

# $TAVERN_BACKUP_PASSPHRASE for scheduled runs; otherwise asked twice, because a
# mistyped passphrase makes the backup unreadable.
function Read-BackupPassphrase {
    $fromEnv = [Environment]::GetEnvironmentVariable($script:PassphraseEnvVar)
    if ($fromEnv) {
        # Child processes (npx, the Supabase CLI) must not inherit it.
        [Environment]::SetEnvironmentVariable($script:PassphraseEnvVar, $null)
        Assert-PassphraseLength $fromEnv
        return $fromEnv
    }
    $first = Get-PassphraseText (Read-Host 'Backup passphrase' -AsSecureString)
    Assert-PassphraseLength $first
    $second = Get-PassphraseText (Read-Host 'Repeat the passphrase' -AsSecureString)
    if ($first -cne $second) { throw 'The two passphrases differ.' }
    return $first
}

function Protect-BackupBytes([byte[]]$plain, [string]$passphrase) {
    Assert-PassphraseLength $passphrase
    $salt = [Security.Cryptography.RandomNumberGenerator]::GetBytes($script:TlencSaltBytes)
    $nonce = [Security.Cryptography.RandomNumberGenerator]::GetBytes($script:TlencNonceBytes)
    $key = [Security.Cryptography.Rfc2898DeriveBytes]::Pbkdf2(
        [Text.Encoding]::UTF8.GetBytes($passphrase), $salt, $script:TlencIterations,
        [Security.Cryptography.HashAlgorithmName]::SHA256, 32)
    $cipher = New-Object byte[] $plain.Length
    $tag = New-Object byte[] $script:TlencTagBytes
    $aes = [Security.Cryptography.AesGcm]::new($key, $script:TlencTagBytes)
    try { $aes.Encrypt($nonce, $plain, $cipher, $tag) } finally { $aes.Dispose() }
    $blob = New-Object byte[] ($script:TlencMagic.Length + $salt.Length + $nonce.Length + $tag.Length + $cipher.Length)
    $at = 0
    foreach ($part in @($script:TlencMagic, $salt, $nonce, $tag, $cipher)) {
        [Buffer]::BlockCopy($part, 0, $blob, $at, $part.Length)
        $at += $part.Length
    }
    return , $blob
}

# Reads the same format as decrypt-backups.ps1 (used by the tests).
function Unprotect-BackupBytes([byte[]]$blob, [string]$passphrase) {
    $head = $script:TlencMagic.Length
    $min = $head + $script:TlencSaltBytes + $script:TlencNonceBytes + $script:TlencTagBytes
    if ($blob.Length -lt $min -or
        [Text.Encoding]::ASCII.GetString($blob, 0, $head) -ne 'TLENC1') {
        throw 'Not a Tavern Ledger encrypted backup.'
    }
    $salt = [byte[]]$blob[$head..($head + 15)]
    $nonce = [byte[]]$blob[($head + 16)..($head + 27)]
    $tag = [byte[]]$blob[($head + 28)..($head + 43)]
    $cipher = New-Object byte[] ($blob.Length - $min)
    [Array]::Copy($blob, $min, $cipher, 0, $cipher.Length)
    $key = [Security.Cryptography.Rfc2898DeriveBytes]::Pbkdf2(
        [Text.Encoding]::UTF8.GetBytes($passphrase), $salt, $script:TlencIterations,
        [Security.Cryptography.HashAlgorithmName]::SHA256, 32)
    $plain = New-Object byte[] $cipher.Length
    $aes = [Security.Cryptography.AesGcm]::new($key, $script:TlencTagBytes)
    try { $aes.Decrypt($nonce, $cipher, $tag, $plain) } finally { $aes.Dispose() }
    return , $plain
}

# A hard kill skips the finally of Invoke-EncryptedBackup; sweep what it left.
function Remove-StaleTempFolders {
    Get-ChildItem ([IO.Path]::GetTempPath()) -Directory -Filter 'tl-backup-*' -ErrorAction SilentlyContinue |
        Where-Object { $_.LastWriteTimeUtc -lt (Get-Date).ToUniversalTime().AddDays(-1) } |
        ForEach-Object { Remove-Item -Recurse -Force -Confirm:$false $_.FullName -ErrorAction SilentlyContinue }
}

# Runs $Work with a fresh folder under the system temp folder (outside the
# repo), zips that folder, encrypts the zip to $OutFile and always deletes the
# temp folder, also when $Work or the encryption throws. A failed run leaves no
# output file either.
function Invoke-EncryptedBackup {
    param(
        [Parameter(Mandatory)][string]$OutFile,
        [Parameter(Mandatory)][string]$FolderName,
        [Parameter(Mandatory)][AllowEmptyString()][string]$Passphrase,
        [Parameter(Mandatory)][scriptblock]$Work
    )
    Assert-PassphraseLength $Passphrase
    Remove-StaleTempFolders
    $temp = Join-Path ([IO.Path]::GetTempPath()) ("tl-backup-" + [guid]::NewGuid().ToString('N'))
    $partial = "$OutFile.partial"
    try {
        New-Item -ItemType Directory -Force $temp | Out-Null
        # Only the current user may read the temp folder while it holds plaintext.
        if ($IsWindows) {
            & icacls $temp /inheritance:r /grant:r "$($env:USERNAME):(OI)(CI)F" | Out-Null
            if ($LASTEXITCODE -ne 0) { throw 'Could not restrict access to the temp folder.' }
        }
        else { [IO.File]::SetUnixFileMode($temp, 'UserRead, UserWrite, UserExecute') }
        $dir = Join-Path $temp $FolderName
        New-Item -ItemType Directory -Force $dir | Out-Null
        & $Work $dir

        $zip = Join-Path $temp 'backup.zip'
        [IO.Compression.ZipFile]::CreateFromDirectory($dir, $zip, 'Optimal', $true)
        $blob = Protect-BackupBytes ([IO.File]::ReadAllBytes($zip)) $Passphrase
        New-Item -ItemType Directory -Force (Split-Path -Parent $OutFile) | Out-Null
        [IO.File]::WriteAllBytes($partial, $blob)
        Move-Item -Force $partial $OutFile
    }
    finally {
        Remove-Item -Recurse -Force -Confirm:$false $temp -ErrorAction SilentlyContinue
        if (Test-Path $temp) { Write-Warning "Could not delete the temp folder $temp; it holds plaintext, delete it by hand." }
        Remove-Item -Force -Confirm:$false $partial -ErrorAction SilentlyContinue
    }
}

function Test-StampOlderThan([string]$stamp, [int]$keepDays) {
    $parsed = [datetime]::MinValue
    $ok = [datetime]::TryParseExact($stamp, 'yyyyMMdd-HHmmss',
        [Globalization.CultureInfo]::InvariantCulture, [Globalization.DateTimeStyles]::None, [ref]$parsed)
    return $ok -and $parsed -lt (Get-Date).ToUniversalTime().AddDays(-$keepDays)
}

# Deletes tl-backups-<stamp>.tlenc files older than $KeepDays in $OutDir, and
# the plaintext <stamp> folders that older script versions left in $LegacyDir
# (they keep expiring until the owner removes them). Nothing else is touched.
function Remove-ExpiredBackups([string]$OutDir, [string]$LegacyDir, [int]$KeepDays) {
    Get-ChildItem $OutDir -File -Filter 'tl-backups-*.tlenc' -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -match '^tl-backups-(\d{8}-\d{6})\.tlenc$' -and (Test-StampOlderThan $Matches[1] $KeepDays) } |
        ForEach-Object {
            Write-Host "Deleting backup older than $KeepDays days: $($_.Name)"
            Remove-Item -Force -Confirm:$false $_.FullName
        }
    Get-ChildItem $LegacyDir -Directory -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -match '^\d{8}-\d{6}$' -and (Test-StampOlderThan $_.Name $KeepDays) } |
        ForEach-Object {
            Write-Host "Deleting old plaintext backup folder: $($_.Name)"
            Remove-Item -Recurse -Force -Confirm:$false $_.FullName
        }
}

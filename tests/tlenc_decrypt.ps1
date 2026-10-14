# Copy of the format logic in the user's decrypt-backups.ps1 (TLENC1), with the
# passphrase taken from $env:TLENC_TEST_PASSPHRASE instead of a prompt.
param([Parameter(Mandatory)][string]$File, [Parameter(Mandatory)][string]$Zip)
$ErrorActionPreference = 'Stop'
$all = [IO.File]::ReadAllBytes($File)
if ([Text.Encoding]::ASCII.GetString($all[0..5]) -ne 'TLENC1') { throw 'Not a Tavern Ledger encrypted backup.' }
$s = [byte[]]$all[6..21]; $n = [byte[]]$all[22..33]; $t = [byte[]]$all[34..49]; $c = [byte[]]$all[50..($all.Length - 1)]
$pw = $env:TLENC_TEST_PASSPHRASE
$key = [Security.Cryptography.Rfc2898DeriveBytes]::Pbkdf2([Text.Encoding]::UTF8.GetBytes($pw), $s, 600000, [Security.Cryptography.HashAlgorithmName]::SHA256, 32)
$plain = New-Object byte[] $c.Length
[Security.Cryptography.AesGcm]::new($key, 16).Decrypt($n, $c, $t, $plain)
[IO.File]::WriteAllBytes($Zip, $plain)

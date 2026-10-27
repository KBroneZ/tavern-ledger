# winget manifests

The [Windows Package Manager](https://learn.microsoft.com/windows/package-manager/) community repository ([microsoft/winget-pkgs](https://github.com/microsoft/winget-pkgs)) lists a package as one folder of YAML manifests per version. `manifests/` here is that folder for the newest release, laid out as winget-pkgs expects (`manifests/k/KBroneZ/TavernLedger/<version>/`): the version manifest, the installer manifest and the English (default) and Spanish locale manifests.

The files are made by [`tools/winget_manifest.py`](../../tools/winget_manifest.py) from [`web/src/lib/site/releases.json`](../../web/src/lib/site/releases.json), the record of the published release with GitHub's own SHA-256 of the installer. Do not edit them by hand: a test fails when they differ from what the script makes, or when a folder for an older version is left behind.

```
python tools/winget_manifest.py          # after `python tools/release.py facts vX.Y.Z`
python tools/winget_manifest.py --check
winget validate --manifest packaging/winget/manifests/k/KBroneZ/TavernLedger/X.Y.Z
```

## What the manifests say

- `InstallerType: nullsoft`, `Scope: user`: the per-user NSIS installer, which winget runs silently with `/S`.
- `UpgradeBehavior: install`: a newer installer goes over the old version and keeps the games and settings.
- `RequireExplicitUpgrade: true`: the app updates itself, so `winget upgrade --all` leaves it alone; `winget upgrade KBroneZ.TavernLedger` still works.
- The descriptions say what the app reads, including the memory read that is on by default, and carry the fan notice.

## Submitting (the repository owner only)

Listing the package in winget-pkgs is a pull request to Microsoft's repository and needs the Microsoft Contributor License Agreement, so only the owner submits it. For the first version, from a clone of this repository on Windows:

```
winget install Microsoft.WingetCreate
wingetcreate submit --prtitle "New package: KBroneZ.TavernLedger version X.Y.Z" packaging/winget/manifests/k/KBroneZ/TavernLedger/X.Y.Z
```

`wingetcreate` asks for a GitHub sign-in, forks winget-pkgs and opens the pull request; sign the CLA when its bot asks, and answer the moderators' questions on the pull request. For later versions, run the script after recording the release and submit the new folder the same way (title `New version: KBroneZ.TavernLedger version X.Y.Z`).

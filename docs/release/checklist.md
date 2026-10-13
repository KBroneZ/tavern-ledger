# Release checklist

How a new version of the Windows app goes out. The installer is built and signed by GitHub Actions ([`.github/workflows/release.yml`](../../.github/workflows/release.yml)), never on a developer's PC, and the release stays a draft until a person publishes it. Apps from v0.2.0 on update themselves from the published release: the app checks `releases/latest/download/latest.json`, downloads the installer and keeps it only if its signature matches the public key built into the app and names the announced version.

## Before the first self-updating release (once)

- [ ] The repository owner has set up GitHub: Immutable releases on, a ruleset for `v*` tags (no update, no deletion), and a `release` Environment with a required reviewer that holds the signing key and its passphrase as secrets.
- [ ] `plugins.updater.pubkey` in [`crates/desktop/tauri.conf.json`](../../crates/desktop/tauri.conf.json) is the release public key (until then the app's updater is off and the release workflow stops at its first step).
- [ ] The privacy policy ([`docs/legal/privacy.md`](../legal/privacy.md) and its Spanish copy) says, in the same pull request that adds the key, that the app asks GitHub for updates when it starts and every few hours, unless "Check for updates automatically" is off, and that this tells GitHub the PC's IP address and the app's updater user agent.

## 1. Prepare the version

- [ ] `main` is green and has everything this version ships.
- [ ] One pull request sets the new version in both [`crates/desktop/tauri.conf.json`](../../crates/desktop/tauri.conf.json) (`version`) and [`crates/desktop/Cargo.toml`](../../crates/desktop/Cargo.toml) (and `Cargo.lock`). Versions only go up: the app never installs an older or equal one.
- [ ] The release notes are written: what changed, in plain words, for players.

## 2. Tag and build

- [ ] Tag the merged commit on `main` as `vX.Y.Z` (the same version) and push the tag. The workflow checks that the tag, the config and the crate agree and that the public key is set.
- [ ] The **build** job makes the installer with `npm run installer` (`tauri build --no-sign -- --locked`) from a clean checkout and writes `SHA256SUMS`.
- [ ] The **sign** job waits for approval in the `release` Environment. **Approve only a run you started**, for the tag you pushed. If a request comes that you did not expect, reject it.
- [ ] The **verify** job checks both signatures against the public key in the tag's own config and the version, and writes `latest.json` and `SHA256SUMS.minisig`.
- [ ] The **draft** job records where the installer was built (build provenance) and creates the release as a **draft**.

If any job fails, nothing is published. Fix the cause and run the failed jobs again; if the tag itself was wrong, delete the draft (if any) and release a new patch version (tags under the ruleset cannot be moved or deleted).

## 3. Review the draft

- [ ] Files: `TavernLedger_X.Y.Z_x64-setup.exe`, its `.sig`, `latest.json`, `SHA256SUMS`, `SHA256SUMS.minisig`, and nothing else.
- [ ] The SHA-256 in the draft's notes equals the installer's line in `SHA256SUMS` and the build job's log.
- [ ] `latest.json` names version X.Y.Z and points to this tag's installer.
- [ ] Download the installer, install it over the previous version on a test PC or account, and check that the app starts, keeps the history and settings, and shows the new version in Settings > Updates.
- [ ] Replace the draft's notes with the release notes. Leave "Set as a pre-release" off and "Set as the latest release" on: the apps look for updates in the latest release only.

## 4. Publish

- [ ] Publish the draft. With Immutable releases, its files and tag can no longer change.

## 5. After publishing

- [ ] `python tools/release.py facts vX.Y.Z` records the published files' sizes and SHA-256 from GitHub into `web/src/lib/site/releases.json`; commit it in a pull request (the download page and the home page's button follow it).
- [ ] The download page's "Updating" words say that the app updates itself (from the first self-updating release on).
- [ ] The next daily **Release files** run ([`.github/workflows/release-check.yml`](../../.github/workflows/release-check.yml)) is green.
- [ ] An app on the previous version shows "Version X.Y.Z is ready" within a minute of starting (or after "Check now"), and updates when "Restart now" is pressed.

## If something goes wrong

- **A bad version is out:** publish a fixed, higher version. To stop the bad one from spreading first, mark the previous release as the latest on GitHub: apps that have not updated stop seeing it; apps that have it keep it (no downgrades) until the fix arrives.
- **The signing key has to change:** a release made with the old key can carry a new public key, and later releases are signed with the new key; nobody reinstalls. If the old key is lost or exposed, apps cannot be moved to a new key safely: every user installs the next version by hand once.

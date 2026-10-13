# Release checklist

How a new version of the Windows app goes out. The installer is built and signed by GitHub Actions ([`.github/workflows/release.yml`](../../.github/workflows/release.yml)), never on a developer's PC, and the release stays a draft until a person publishes it. Apps from v0.2.0 on update themselves from the published release: the app checks `releases/latest/download/latest.json`, downloads the installer and keeps it only if its signature matches the public key built into the app and names the announced version.

## Before the first self-updating release (once)

- [ ] The repository owner has set up GitHub: Immutable releases on; a ruleset for `v*` tags (only the owner may create one; nobody may move or delete one); and a `release` Environment for `v*` tags only, with a required reviewer, holding the signing key and its passphrase as its own secrets (no repository or organization secret of the same name).
- [ ] `plugins.updater.pubkey` in [`crates/desktop/tauri.conf.json`](../../crates/desktop/tauri.conf.json) is the release public key (until then the app's updater is off and the release workflow stops at its first step).
- [ ] The privacy policy ([`docs/legal/privacy.md`](../legal/privacy.md) and its Spanish copy) says, in the same pull request that adds the key, that the app asks GitHub for updates when it starts and every few hours, unless "Check for updates automatically" is off, and that this tells GitHub the PC's IP address and the app's updater user agent.

## 1. Prepare the version

- [ ] `main` is green and has everything this version ships.
- [ ] One pull request sets the new version in both [`crates/desktop/tauri.conf.json`](../../crates/desktop/tauri.conf.json) (`version`) and [`crates/desktop/Cargo.toml`](../../crates/desktop/Cargo.toml) (and `Cargo.lock`). Versions only go up: the app never installs an older or equal one.
- [ ] The release notes are written: what changed, in plain words, for players.

## 2. Tag and build

- [ ] Tag the merged commit on `main` as `vX.Y.Z` (the same version) and push the tag. The workflow checks that the tag, the config and the crate agree, that the commit is on `main` and that the public key is set.
- [ ] The **build** job makes the installer with `npm run installer` (`tauri build --no-sign -- --locked`) from a clean checkout and writes `SHA256SUMS`.
- [ ] The **sign** job waits for approval in the `release` Environment. **Approve only a run you started**, for the tag you pushed. If a request comes that you did not expect, reject it. The job checks out nothing: it signs the built bytes with a Tauri CLI pinned in the workflow by SHA-512 (bumping that CLI is a reviewed change to the workflow, separate from `package-lock.json`).
- [ ] The **verify** job checks both signatures and their version against the key installed apps trust (the previous release's; the tag's own for the first self-updating release), and writes `latest.json` and `SHA256SUMS.minisig`.
- [ ] The **draft** job records where the installer was built (build provenance) and creates the release as a **draft**.

If any job fails, nothing is published. Fix the cause and run the failed jobs again; if the tag itself was wrong, delete the draft (if any) and release a new patch version (tags under the ruleset cannot be moved or deleted).

## 3. Review the draft

- [ ] If the draft's notes start with "WARNING: this release changes the updater public key" and you did not plan a key rotation, delete the draft and do not publish.
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
- [ ] The next daily **Release files** run ([`.github/workflows/release-check.yml`](../../.github/workflows/release-check.yml)) is green. It is a best-effort check: GitHub can skip scheduled runs and turns schedules off in a repository with no activity for 60 days, so look at its runs now and then and turn it back on if needed.
- [ ] An app on the previous version shows "Version X.Y.Z is ready" within a minute of starting (or after "Check now"), and updates when "Restart now" is pressed.

## If something goes wrong

- **A bad version is out:** publish a fixed, higher version. To stop the bad one from spreading first, mark the previous release as the latest on GitHub: apps that have not checked yet stop seeing it. Apps that already have it keep it (no downgrades), and an app that already downloaded it still offers it until it restarts.
- **The signing key has to change:** the release that carries the new public key is signed with the old key (the workflow checks it against the previous release's key), and the releases after it with the new key; nobody reinstalls, as long as that release stays the latest long enough for apps to pick it up: an app that skips it cannot verify the next one and must be reinstalled by hand.
- **The signing key is lost or exposed:** there is no way to withdraw a key from installed apps. Delete the Environment's secrets, make a new key, publish a release with the new public key, and tell everyone on the download page to install it by hand once. Until then, whoever holds the old key and can publish releases here can make an update the apps accept, so the GitHub account and its settings are the line of defence.

## History

- **v0.2.0** (tag only, no release): the first run of the release workflow built, signed and checked the installer, but skipped the draft job, whose condition did not allow for the skipped dry-run job before it. The tag cannot be moved or deleted (the tag ruleset), so it stays without a release. `tools/check_workflow_needs.py` now fails CI on that kind of condition. **v0.2.1** is the first self-updating release; its signatures are checked against the key in v0.2.0's config, which is the same key.

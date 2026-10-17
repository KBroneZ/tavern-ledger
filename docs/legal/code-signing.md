# Code signing policy

Last updated: 17 October 2026

Free code signing provided by [SignPath.io](https://signpath.io), certificate by [SignPath Foundation](https://signpath.org).

Tavern Ledger's Windows installer is signed so that Windows can show who published it and warn only when the file has been changed. This page says what is signed, who decides, and what the signed program does with your data.

## What is signed

The Windows installer (`TavernLedger_X.Y.Z_x64-setup.exe`) of each release. It is built from this repository's source code by GitHub Actions on GitHub's own servers (the workflow is `.github/workflows/release.yml`), never on a developer's PC, from a version tag on the `main` branch. SignPath checks that the file really comes from that build before it signs it.

Every release is approved by hand before it is signed. Nothing else is signed with this certificate, and only software built from this project's own source code is ever signed.

The publisher Windows shows for a signed installer is SignPath Foundation, because the certificate is theirs.

## Team roles

- **Committers and reviewers:** [KBroneZ](https://github.com/KBroneZ), the maintainer. A change from anyone else is reviewed by the maintainer before it is merged.
- **Approvers:** [KBroneZ](https://github.com/KBroneZ). Each signing request is approved by hand.

Everyone with these roles uses multi-factor authentication on GitHub and on SignPath.

## Privacy policy

This policy covers what the program and the website do with your data, including every address the program contacts: [Privacy policy](/privacy/).

## Contact

To report a problem with a signed file, follow the steps in [SECURITY.md](https://github.com/KBroneZ/tavern-ledger/blob/main/SECURITY.md) or write to contact@tavernledger.net.

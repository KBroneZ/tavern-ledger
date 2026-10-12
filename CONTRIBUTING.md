# Contributing to Tavern Ledger

Thanks for wanting to help. Bug reports, ideas, fixes, translations and docs are all welcome.

## License of contributions

Tavern Ledger is licensed under the [GNU Affero General Public License, version 3 only](LICENSE) (`AGPL-3.0-only`). By opening a pull request you agree that your contribution is licensed under the same terms. There is no separate contributor agreement.

## Before you start

- **Open an issue first** for anything bigger than a small fix, so we can agree on the approach before you spend time on it.
- **Bugs:** use the bug report template. Never attach raw Hearthstone logs: they contain other players' names. The app's "Report a problem" file is made for this.
- **Security problems:** do not open a public issue; see [SECURITY.md](SECURITY.md).

## Project rules every change must keep

These are the project's rules and are not negotiable in a pull request:

1. **Clean room.** Do not copy, port or paraphrase code from projects whose license does not allow it, including Hearthstone Deck Tracker, Firestone, HearthMirror, BobsBuddy and their simulators. Nomi's Kitchen is also off limits: its license forbids use with AI tools, and this project uses them. Code under a compatible license (MIT, Apache-2.0, BSD…) is fine when it is recorded in [PROVENANCE.md](PROVENANCE.md).
2. **Only what the player can see.** The app reads the game's log files and, only behind its off-by-default switch, two values the game shows the player. No injection, no changes to game files, no automated actions, and nothing the player could not see on their own screen.
3. **No other players' data.** Test fixtures are synthetic or the maintainer's own games scrubbed by `tools/make_fixture.py`. No names, BattleTags or account ids in code, fixtures or logs.
4. **No secrets** in code, fixtures, logs or commits.
5. **New dependencies** need a license compatible with AGPL-3.0-only and a row in [PROVENANCE.md](PROVENANCE.md); CI checks the Rust and npm ones.
6. **Brand and art:** no Blizzard logos. Card art only under Blizzard's Fan Content Policy, loaded at run time, never committed.
7. **Unknown is not zero.** When the app cannot tell a value, it shows it as unknown, never a guess.

## Making a change

1. Fork the repository and create a branch from `main`.
2. Write tests with the change. Run the checks that apply (full commands in [docs/development.md](docs/development.md)):

   ```
   python -m unittest discover -s tests
   cargo fmt --all --check
   cargo test --workspace --exclude desktop
   cd web && npm ci && npm test && npm run build && npm run test:dist
   ```

3. Write code, comments, docs and commit messages in English. The app and the website are in English and Spanish: a change to the UI's words needs both.
4. Commit messages follow `type: description`, with `feat`, `fix`, `refactor`, `docs`, `test`, `chore`, `perf` or `ci`.
5. Open a pull request against `main` and fill in the template. CI must be green before it can be merged.

## Questions

Ask in an issue, or write to contact@tavernledger.net.

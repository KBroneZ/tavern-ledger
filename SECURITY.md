# Security policy

## Reporting a problem

Please report security problems privately, not in a public issue:

- through GitHub's **Report a vulnerability** button on this repository's [Security tab](https://github.com/KBroneZ/tavern-ledger/security), or
- by email to contact@tavernledger.net, with "Security" in the subject.

The same address is published for automated tools at [`/.well-known/security.txt`](https://tavernledger.net/.well-known/security.txt). English or Spanish is fine.

Say what is affected, how to reproduce it and what an attacker could do with it. You will get a reply by email. Please give us reasonable time to fix the problem before you share details publicly; we will credit you in the fix if you want.

## Scope

- The desktop app and its tools in this repository.
- The website at [tavernledger.net](https://tavernledger.net) and its backend (the database migrations and server functions in `supabase/`).

Out of scope: Hearthstone and Blizzard's services (report those to Blizzard), and problems in third-party services we use that are not caused by how we use them.

## Versions

Version 0.1.0 (beta) is the current release; see the [releases page](https://github.com/KBroneZ/tavern-ledger/releases). Only the latest release gets fixes. A fix goes to `main` and to the live website first, and reaches the desktop app in the next release.

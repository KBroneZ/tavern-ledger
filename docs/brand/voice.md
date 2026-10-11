# Tavern Ledger voice

How the website and the app talk to players (T-533, D-102). Read this before writing or changing any text a player sees. The app's own strings have their review in [app-strings.md](app-strings.md).

## Tone

- One player talking to another: plain, short, friendly, never cute. Say what happened and what to do next.
- English as a native player writes it. Spanish as a player from Spain writes it (informal "tú"), not as a translation: rewrite the sentence, do not map it word by word.
- Facts stay exact: numbers, limits and promises are copied, never rounded up or made nicer. No made-up features, figures or testimonials (rule 8).
- Unknown is not zero: say "unknown" or show a dash; never write a sentence that turns a missing value into 0.
- The fan notice is fixed text. Do not reword it.
- No Blizzard names in the brand. "Hearthstone" only to say what the app is for. Never show BattleTags, account ids, keys or emails.

## Rules of thumb

| Do | Avoid |
|----|-------|
| Headings name the thing ("Your records") | A heading that repeats the line above it (breadcrumb, kicker) |
| Buttons say what they do: verb + object ("Download my data", "Show games") | "OK", "Submit", "Change" on its own |
| Spanish buttons in the infinitive ("Descargar mis datos", "Reintentar") | Mixing "Inténtalo de nuevo" / "Intentarlo de nuevo" / "Reintentar" for the same action |
| Errors: what failed, then what to do ("Could not load your games. Check your connection and try again.") | "Something went wrong" alone, jokes, "Oops" |
| Empty states: why it is empty and how to fill it | "No data" |
| Mode in headings as "Overview · Solo"; in captions as "(Solo)" | "Overview, Solo" (reads like a list) |
| Pages that can show someone else's data (public profiles) use neutral words ("Best values and when they were set") | "you" / "your" on a view a visitor may be reading |
| Name the app's tabs only once the app really has that name | Pointing players to a tab label the app does not show yet |
| Contractions in English sentences are fine; keep each paragraph consistent | Stiff "cannot / does not" chains in friendly text |

## The records page (D-102)

The page that was "Curious stats" / "Estadísticas curiosas" is **Records** / **Récords**. It holds the player's records and their totals.

| Where | English | Spanish |
|-------|---------|---------|
| Page and tab name | Records | Récords |
| Own page title | Your records | Tus récords |
| Public page title | {name}'s records | Récords de {name} |
| Link from the dashboard | All records and totals | Todos los récords y totales |

The address stays `/curious/` (`/es/curious/`): the page is `noindex`, so a new address would only add a redirect. Rejected options, for the user to switch to if wanted: "Records and totals" / "Récords y totales" (exact but long for a tab), "Highlights" / "Lo más destacado" (vague, sounds like marketing).

The heroes page uses the same pattern: "Your heroes", "{name}'s heroes" / "Tus héroes", "Héroes de {name}". The breadcrumb above both titles is only the way back ("Back to your dashboard").

## Before and after (from T-533)

| Before | After | Why |
|--------|-------|-----|
| Volver a tu panel · Estadísticas curiosas / Tus estadísticas curiosas | Volver a tu panel / Tus récords | Odd name; title repeated the breadcrumb |
| Advanced, at the top, adds the economy, combats… | Switch to Advanced at the top to also see economy, combats… | Says what to do |
| Partidas ofrecido | Veces que salió | Broken grammar; players say a hero "sale" |
| Left out of the address, not valid: hero, from. | Some filters in the address were not valid and were ignored: hero, from. | Robotic |
| Said and happened / Dijo y Pasó | Predicted and real / Previsto y Lo que pasó | Literal |
| Boards met | Opponents' boards / Tableros de los rivales | Unclear |
| Lobby / Partida (ES) | Lobby / Héroes de la partida | ES repeated the page's kicker |
| Shop: Extra gold / Oro más allá de los ingresos | Extra gold / Oro extra | ES said something else than EN |
| Being rebuilt / En regeneración | Being updated / Actualizándose | Jargon |
| Autodeclarados por la aplicación… | Los envía la aplicación… | Calque of "self-reported" |
| Heroes are shown by card id. | (removed) | No longer true since D-092 |
| Open source (AGPL-3.0). Not open to the public yet. | Open source (AGPL-3.0). The site is not open to the public yet. | Read as if the code were closed; the repo is public |

## Glossary

Game terms follow the game's own esES client (D-086). Prose uses "vida" for health, as players say it.

| English | Spanish (Spain) | Note |
|---------|-----------------|------|
| Battlegrounds | Campos de Batalla | Never "Battlegrounds" in Spanish text |
| Solo / Duos | Solo / Dúos | Not "Individual" |
| tavern / tavern tier | taberna / nivel de taberna | "mejora de taberna" for a tier up |
| minion | esbirro | |
| Mech (tribe), Venomous, Rally | Robot, Ponzoña, Alentar | The esES game's own strings (checked in T-531) |
| golden | dorado | "(dorado)" after a card name |
| triple | triple | |
| trinket | abalorio | |
| Dark Gift | Don siniestro | |
| Old God | Dios antiguo | |
| roll / free roll | refresco / refresco gratis | masculine: "refrescos pagados" |
| hero power | poder de héroe | |
| skin | aspecto | Counts as its base hero |
| place, 1st | puesto, 1.º | "1.er puesto" in words |
| top 4 / top 2 | top 4 / top 2 | |
| opponent / teammate | rival / compañero | |
| health / attack | vida / ataque | |
| gold / gold from selling / extra gold | oro / oro por ventas / oro extra | |
| gold beyond base income | oro más allá del oro base del turno | The app's wording since T-531 |
| win / tie / loss (result) | victoria / empate / derrota | |
| pick rate, top 4 rate, win rate | porcentaje de elección, porcentaje de top 4, porcentaje de victorias | Not "tasa" |
| offered (hero choice) | salió ("te salió") | |
| inferred (worked out) | deducido | |
| self-reported | lo envía la aplicación del jugador | Not "autodeclarado" |
| overlay | overlay | Not "superposición" |
| the app / the website | la aplicación / la web | Not "la app" or "el sitio" in running text |
| log (the game's log file) | registro | |
| dashboard | panel | |
| leaderboards | clasificaciones | |
| display name | nombre visible | |
| sign in / sign up | iniciar sesión / registrarse | "Entrar" only in the menu |

## Open items for other sessions

**Home page (T-530, copy left as it is while it is redesigned):**

- `home.step5.text` (ES): "tus clics atraviesan hasta el juego" → "tus clics llegan al juego".
- `home.safe.body` (ES): "superposición" → "overlay", as on the rest of the site.
- `home.yours.private.title` (ES): "Privado hasta que tú digas" → "Privado hasta que tú quieras".
- `home.hero.lit`: "We kept your games." reads as if it already happened to a new visitor; consider "Every game, kept." / "Todas tus partidas, guardadas."
- `home.status.title`: "The doors are not open yet." says almost the same as `home.status.closed` ("Accounts are not open yet.") just below it; keep one.

**Leaderboards (session 090 owns `web/src/lib/leaderboards/`):**

- `page.kicker`: "Public" / "Público" → "Community" / "Comunidad", as on hero stats.
- `section.advanced`: "Economy and fun" / "Economía y curiosidades" → "Economy and boards" / "Economía y tableros".
- `page.loading`, `page.loaded`, `page.error` (EN): "the boards" can be read as game boards → "the leaderboards".
- `board.caption`: "{board}, {mode}, {period}" → "{board} · {mode} · {period}".
- `page.intro`, `board.how.source` (ES): "autodeclaradas" → "cifras que envía la aplicación de cada jugador".

**Across the site and the app:**

- "Gold beyond income" / "Oro más allá de los ingresos" is hard to read in both languages. A better pair is "Gold on top of income" / "Oro por encima de los ingresos", but it has to change in the dashboard, records, leaderboards and the app at once.
- "Place in each of the last 1 games": the trend chart's text has no singular form (`chart.trend.desc`).
- The Spanish legal pages were reworded in T-533 without changing their meaning; a wider rewrite would need the user's review.

# Viabilidad de un tracker solo para Battlegrounds

Fecha: 2026-10-08. Research de ChatGPT (codex, con web) contrastado por Claude con GitHub, npm y webs oficiales. Lo marcado "sin verificar" no se ha comprobado en fuente primaria.

## Cómo obtienen los datos los trackers existentes

- `Power.log` (activado con `log.config`) más lectura de memoria del cliente: HDT con HearthMirror, Firestone con MindVision.
- Overwolf deprecó los eventos de juego (GEP) de Hearthstone: fecha 6-jul-2026, retirada prevista 10-ago-2026, "lack of usage/players". [Overwolf](https://dev.overwolf.com/ow-native/live-game-data-gep/supported-games/deprecated/overview/)

## Licencias (verificadas)

| Componente | Licencia | ¿Reutilizable? |
|------------|----------|----------------|
| [python-hearthstone](https://github.com/HearthSim/python-hearthstone) | MIT | Sí |
| [python-hslog](https://github.com/HearthSim/python-hslog) | MIT | Sí (parser de `Power.log`) |
| [hs-game-converter-csharp-port](https://github.com/Zero-to-Heroes/hs-game-converter-csharp-port) (Firestone) | MIT | Sí |
| [HSTracker](https://github.com/HearthSim/HSTracker) (macOS) | MIT | Sí, con cuidado: hereda mucho de HDT |
| [hearthstone-battlegrounds-simulator](https://github.com/twanvl/hearthstone-battlegrounds-simulator) | MIT | Sí, pero hay que revisar si está al día |
| [HearthstoneJSON](https://hearthstonejson.com/) | Web CC0; datos de cartas © Blizzard | Datos sí, arte según Fan Content Policy |
| [HDT](https://github.com/HearthSim/Hearthstone-Deck-Tracker) | "All Rights Reserved" (README) | No |
| [Firestone](https://github.com/Zero-to-Heroes/firestone) | Sin licencia; ToS restrictivos | No |
| [@firestone-hs/simulate-bgs-battle](https://www.npmjs.com/package/@firestone-hs/simulate-bgs-battle) | "All rights reserved" (v1.1.770, actualizado 8-oct-2026) | No |
| HearthMirror, BobsBuddy (HearthSim) | Repos privados (404) | No |
| Nomi's Kitchen (HDT y Firestone) | "MIT NON-AI": prohíbe usar el código con IA | No leer ni portar |

## Blizzard

- El EULA prohíbe procesos no autorizados que intercepten o extraigan información; Blizzard puede permitir interfaces de terceros. [EULA](https://www.blizzard.com/es-es/legal/588783f5-79da-4e1c-89dd-ebe212764dda/contrato-de-licencia-para-usuario-final-de-blizzard)
- El reglamento de Lobby Legends 2022 permite trackers en torneos si muestran solo lo que el jugador podría ver. [Rulebook](https://assets.blz-contentstack.com/v3/assets/bltc965041283bac56c/bltb47fca8192f92a36/622fc8f9da5a7125ed751ae9/Battlegrounds_Lobby_Legends_Rulebook_V_1.4.pdf)
- No se encontró fuente fiable de funciones de BG retiradas a petición de Blizzard.
- Desde el parche 33.2 (1-ago-2025) el juego trae una guía de esbirros y hechizos por tier y tipo. [Parche 33.2](https://hearthstone.blizzard.com/en-us/news/24223019)

## MMR: leaderboard público

Endpoint que usa la propia web: `https://hearthstone.blizzard.com/en-gb/api/community/leaderboardsData?region=EU&leaderboardId=battlegrounds&page=1` (no documentado oficialmente; puede cambiar).

Consulta del 8-oct-2026:

| Leaderboard | Cuentas |
|-------------|---------|
| BG Solo EU | 4294 (el último, con 8000 de MMR) |
| BG Solo US | 1954 |
| BG Solo AP | 1821 |
| BG Duos EU | 1480 |

Conclusión: solo cubre el top. La mayoría de jugadores no aparece, así que para ellos hace falta otra fuente (P-007). Columnas: `rank`, `accountid` (BattleTag sin número), `rating`; varios jugadores pueden compartir nombre.

## Competencia

- **Firestone:** el más completo (overlay, historial, MMR, simulador, replays en el cliente anunciados el 23-sep-2026, versión independiente en beta). Gratis con anuncios + Premium (precio no recuperado).
- **HDT + HSReplay Tier7:** Bob's Buddy gratis; Tier7 de pago. Según ChatGPT, $25 por 6 meses (sin verificar: la página devolvió 403).
- **HSGuru:** donaciones, sin paywall; no se confirmó overlay de BG propio.
- **BG Know-How:** abandonado en julio de 2025 "due to time constrains" (verificado en su README).
- **Nomi.gg / Nomi's Kitchen:** plugin gratis para HDT y Firestone, más beta de Android. Su "Fix Minion Dance" y "Disable abbreviation" van en un plugin **BepInEx** que se inyecta en el cliente del juego.

## Esfuerzo estimado (estimación de ChatGPT, no son datos)

| Alcance | Construir | Mantener |
|---------|-----------|----------|
| Prueba de viabilidad | 40–80 h | — |
| MVP sin simulador | 250–500 h | 8–20 h/mes |
| Producto sólido sin simulador | 600–1.200 h | 20–40 h/mes |
| Completo con simulador | 1.500–3.000+ h | 40–100+ h/mes |

## Firma de código (Windows)

| Opción | Precio | Notas |
|--------|--------|-------|
| [SignPath Foundation](https://signpath.org/) | Gratis | Solo open source; build automatizada desde el repo; clave en HSM de SignPath. |
| [Certum](https://shop.certum.eu/code-signing.html) Open Source in the Cloud | desde 49 € | Para software libre. |
| Certum Standard (cloud / tarjeta / código) | desde 209 / 169 / 139 € | OV. Desde el 27-feb-2026, validez máxima de 459 días por certificado (reemisiones gratis en planes de 2–3 años). |
| Certum EV | desde 329–379 € | Innecesario para este proyecto. |
| [Azure Artifact Signing](https://learn.microsoft.com/en-us/azure/artifact-signing/faq) | Precio no visible en la web | Particulares solo en EE. UU. y Canadá: no vale para un particular en España. |

Ni la firma OV ni la EV evitan el aviso de SmartScreen al principio: la reputación se gana con descargas (sin verificar en fuente primaria para 2026).

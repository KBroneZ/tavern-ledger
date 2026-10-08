# bg-tracker — tracker de Hearthstone Battlegrounds

Proyecto personal (nombre provisional): app de escritorio para Windows que lee el log de Hearthstone y una web para seguir el progreso. Gratis por defecto, con extras de pago opcionales. Open source (D-002).

**Equipo:** el usuario (único humano: decide, aprueba, publica) + Claude Code (ingeniería) + ChatGPT Plus (research y segunda opinión con la skill `ask-chatgpt`).

Reglas portadas de StartAICareer (`C:\Users\andia\StartAICareer\CLAUDE.md`) y adaptadas. Si algo no está aquí, no se hereda.

## Protocolo de sesión

1. Al empezar: leer `README.md` (estado) y la tarea en `docs/plan/plan.md`. Consultar `docs/decisions/DECISIONS.md` si la tarea toca algo pendiente.
2. Una tarea por conversación. Título de la conversación: `#NNN <nombre>` (numeración propia de este repo).
3. Al terminar: estado de la tarea en el plan, decisiones nuevas en `DECISIONS.md` y README al día, en el mismo commit o PR de la tarea.
4. Preguntar si se quiere el prompt de la siguiente sesión; crearlo solo si el usuario lo pide, en `prompts/sesiones/NNN-slug.md`, con effort por fase y modelo de subagentes (Haiku para búsquedas, Sonnet para código, Opus solo para arquitectura, seguridad o decisiones irreversibles).

## Reglas absolutas

1. **Clean-room.** Nunca entra material del empleador del usuario ni código de terceros con licencia incompatible. El código de HDT ("All Rights Reserved"), Firestone (sin licencia), sus simuladores, HearthMirror y BobsBuddy **no se copian ni se portan**. El código de Nomi's Kitchen (licencia "MIT NON-AI") **no se lee ni se pasa a ninguna IA**. Se reutiliza solo lo que tiene licencia compatible (MIT, Apache-2.0…) y queda anotado en `PROVENANCE.md`. Duda → parar y avisar.
2. **Solo equipo y cuentas personales.**
3. **Límites con Blizzard (D-004):** el MVP solo lee archivos de log locales. Nada de lectura de memoria, inyección (BepInEx o similar), modificar archivos del juego, automatizar acciones ni mostrar información que el jugador no podría ver en su pantalla. Cualquier excepción necesita una decisión explícita del usuario en `DECISIONS.md`.
4. **APIs ajenas con respeto:** el leaderboard público de Blizzard se consulta con caché, pocas peticiones, timeout y User-Agent identificable. Nada de scraping que no sea de endpoints públicos que la propia web usa, ni a ritmo agresivo.
5. **Marca y arte:** ni logos ni nombre de Blizzard en la marca. Las imágenes de cartas, solo según la Fan Content Policy de Blizzard. Siempre el aviso "Unofficial fan project. Not affiliated with or endorsed by Blizzard Entertainment."
6. **Datos de usuarios (RGPD):** mínimos, con política de privacidad, exportación y borrado de cuenta desde el primer día que haya servidor. Nada de vender datos.
7. **Secrets** nunca en código, fixtures, logs ni commits. `.env` y `.local/` en `.gitignore`.
8. **Marketing:** nunca inventar capacidades, cifras ni testimonios. No decir "aprobado por Blizzard".
9. **Acciones externas** (crear el repo en GitHub, publicar, pagar, crear cuentas, aceptar términos, push) → solo con confirmación explícita del usuario. Nunca push directo a `main` ni force-push una vez exista remoto.

## Principios de ingeniería

- `main` siempre liberable. Tests antes o junto al código.
- Nunca declarar "hecho" sin evidencia (tests, output, diff).
- Vacío ≠ desconocido ≠ cero. Un fallo silencioso es un defecto. Si un parche de Hearthstone rompe el parser, la app lo dice ("versión no soportada") en vez de mostrar datos falsos.
- Fixtures: solo logs propios del usuario o sintéticos, sin datos de otros jugadores que no sean públicos.
- Toda integración de red: timeout, errores manejados, validación de respuesta, logging.
- Respuestas externas y output de IA = input no confiable hasta verificar.
- Toda dependencia con origen y licencia en `PROVENANCE.md`.
- Simple > ingenioso. Sin abstracciones especulativas. Código determinista antes que LLM.

## Riesgo y review

| Tier | Ejemplos | Review |
|------|----------|--------|
| R0 | docs, copy | autoreview |
| R1 | código normal | TDD + `/code-review` |
| R2 | red, auth, datos de usuarios, dependencias, instalador | `/code-review` + `/security-review` |
| R3 | pagos, secrets, producción, firma de código | lo de R2 + segunda IA (`/santa-loop` o ChatGPT) + OK del usuario |

## Comunicación

- Chat en `/caveman full` (regla global), en español.
- Docs, código, commits y PRs en lenguaje normal y conciso: docs en español; producto, UI y README público en inglés.
- Research largo → `ask-chatgpt` en segundo plano.

## Mapa

```
CLAUDE.md                reglas (este archivo)
README.md                estado actual
PROVENANCE.md            origen y licencia de dependencias y datos
docs/plan/plan.md        fases y tareas
docs/decisions/          decisiones tomadas y pendientes
docs/research/           investigación verificada
prompts/sesiones/        prompts de sesión (solo si el usuario los pide)
.local/                  solo local, nunca en git
```

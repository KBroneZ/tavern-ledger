# Sesión 001: Arranque — nombre, repo en GitHub y logs

Modelo: sonnet
Effort: medium
Subagentes: haiku

Trabaja en `C:\Users\andia\bg-tracker` siguiendo su `CLAUDE.md`: caveman en el chat; documentos en español; README público y producto en inglés. **Tier R0–R1** (docs, configuración y un script de comprobación de logs; el push y la creación del repo son acciones externas que necesitan el OK del usuario).

**Línea roja (D-004, D-006):** solo lectura de archivos de log locales. Nada de inyección, BepInEx, DLLs, lectura de memoria, modificar archivos del juego ni automatizar acciones. Si una tarea parece necesitarlo, se para y se avisa.

## Effort y modelos por fase

| Fase | Effort | Subagentes (si hacen falta) |
|------|--------|-----------------------------|
| Lectura y plan | low | Haiku (búsquedas) |
| Nombre (research de disponibilidad) | medium | `ask-chatgpt` en segundo plano para comprobar nombres, dominios y marcas |
| Implementación (script de logs, repo) | medium | Sonnet |
| Review | medium | — (autoreview + `/code-review` del script) |
| Docs (plan, decisiones, README) | low | — |

**Autonomía:** Claude propone nombres y comprueba disponibilidad; **el usuario elige el nombre** y **aprueba crear el repo en GitHub y hacer push**. Claude decide la estructura del script y de la CI.

## Al empezar

1. Título **`#001 Arranque repo y logs`**.
2. Lee: `README.md`, `docs/plan/plan.md` (T-001 y T-002), `docs/decisions/DECISIONS.md` (P-001, D-001 a D-006), `docs/research/viabilidad.md`.

## Tarea

1. **Nombre (P-001):** proponer 5–8 nombres sin "Hearthstone", "Blizzard" ni marcas del juego. Comprobar que no los usa otro tracker conocido, que el nombre está libre en GitHub y si hay dominio `.gg` o `.app` libre (sin comprar nada). Preguntar al usuario cuál elige.
2. **Repo (T-001):** con el OK del usuario, renombrar la carpeta si hace falta, crear el repo **público** en GitHub (las Actions son gratis en repos públicos) y hacer push de `main`. A partir de ahí, ramas y PR, nunca push directo a `main`.
3. **CI mínima:** workflow que valide los enlaces Markdown y busque secretos (p. ej. gitleaks con versión fijada). Anotar cada acción y su versión en `PROVENANCE.md`.
4. **Logs (T-002, parte local):** script de solo lectura que compruebe si `log.config` de Hearthstone activa el log `Power` y dónde se escriben los logs en este equipo. Indicar al usuario qué tiene que activar, pero **no tocar archivos del juego ni de su configuración sin su OK**. Si el usuario ya tiene logs de partidas de BG, inventariarlos (fecha, tamaño, modo) sin copiarlos aún al repo.
5. Actualizar el plan (T-001 hecho; T-002 en curso), `DECISIONS.md` (P-001 → decisión) y el README.

## Fuera de alcance

- Parser y prototipo con `hslog` (T-003, siguiente sesión).
- Elegir stack (P-002), web o backend.
- Comprar dominio, firma de código o cualquier gasto.

## Criterios de aceptación

- [ ] Nombre elegido por el usuario y anotado en `DECISIONS.md`.
- [ ] Repo público en GitHub con README, LICENSE MIT, `.gitignore` y la CI en verde (enlace al run).
- [ ] Script de comprobación de logs con su salida real como evidencia; sin escribir en carpetas del juego.
- [ ] Plan, decisiones y README al día en el mismo PR.

# Plan

Estados: `pendiente` · `en curso` · `hecho` · `bloqueado`.

## F0 — Prueba de viabilidad (40–80 h estimadas)

Objetivo: demostrar que se puede reconstruir una partida de Battlegrounds solo con logs locales.

| ID | Tarea | Estado | Criterio de aceptación |
|----|-------|--------|------------------------|
| T-001 | Elegir nombre (P-001) y crear el repo en GitHub (con OK del usuario) | pendiente | Repo remoto con README, LICENSE MIT y `.gitignore`. |
| T-002 | Activar logs (`log.config`) y recoger 10+ partidas propias como fixtures (Solo y Duos) | pendiente | Fixtures en `fixtures/` sin datos de terceros que no sean públicos; anotados en `PROVENANCE.md`. |
| T-003 | Prototipo de parser con `hslog` (Python, MIT) para ver qué expone `Power.log` en BG | pendiente | Informe: héroe, tribus del lobby, tableros por ronda, rivales, puesto final, Duos. Qué falta. |
| T-004 | Investigar fuentes de MMR (P-007): logs locales + leaderboard público | pendiente | Tabla de qué dato sale de dónde, con evidencia. |
| T-005 | Decidir stack de escritorio (P-002) | pendiente | Decisión en `DECISIONS.md` con motivo. |
| T-006 | Cliente del leaderboard con caché y límites | pendiente | Tests con respuestas grabadas; timeout, errores y "sin datos" explícitos. |

## F1 — Historial local y web mínima

| ID | Tarea | Estado |
|----|-------|--------|
| T-101 | App de escritorio: vigilar `Power.log` y guardar partidas en local | pendiente |
| T-102 | Stats personales: héroes, puestos medios, tribus | pendiente |
| T-103 | Decidir stack web y backend (P-003) | pendiente |
| T-104 | Cuentas, subida de partidas, perfil público; privacidad y borrado (RGPD) | pendiente |
| T-105 | Firma de código (P-005) e instalador con autoactualización | pendiente |

## F2 — Replays y recaps

| ID | Tarea | Estado |
|----|-------|--------|
| T-201 | Visor de partida turno a turno en la web (tableros vistos de los rivales) | pendiente |
| T-202 | Recap tras cada partida | pendiente |
| T-203 | Récord contra cada rival del lobby | pendiente |

## F3 — Overlay mínimo

| ID | Tarea | Estado |
|----|-------|--------|
| T-301 | Overlay con tribus del lobby, último tablero visto de cada rival y récord | pendiente |

## F4 — Comunidad y extras

| ID | Tarea | Estado |
|----|-------|--------|
| T-401 | Stats agregadas de comunidad (solo con consentimiento y volumen suficiente) | pendiente |
| T-402 | Extras de pago (P-006) | pendiente |

## Fuera de alcance por ahora

- Simulador de combate (coste muy alto; solo si el proyecto sigue vivo tras F4).
- Lectura de memoria e inyección en el cliente (D-004), incluido el "minion dance" (P-004).

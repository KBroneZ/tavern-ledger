# Stack de la app de escritorio (T-005, P-002)

Fecha: 2026-10-09. Informe para decidir P-002. La decisión es del usuario; aquí solo hay datos y una recomendación.

## Qué tiene que hacer la app

Sale de las tareas F1–F3 del plan y de los prototipos de F0:

1. **Seguir `Power.log` mientras crece** y reconstruir cada partida (T-101). La lógica cabe en menos de 500 líneas de Python (`tools/parse_bg.py`) y se reescribe a partir de [`parser-hslog.md`](parser-hslog.md), sin `hslog` ni código de otros trackers.
2. **Guardar partidas en local** (SQLite o archivos) y mostrar stats (T-102).
3. **Consultar el leaderboard** con caché y límites (D-012, D-013); el prototipo es `tools/leaderboard.py`.
4. **Subir partidas** a la web cuando haya cuentas (T-104).
5. **Instalador con autoactualización y firma** (T-105, P-005: SignPath Foundation primero).
6. **Overlay mínimo** (T-301): ventana transparente, siempre encima, que deje pasar los clics donde no hay nada.
7. Solo Windows. Mantenible por una persona con Claude Code.

## Opciones

| | Rust + Tauri 2 | C# / .NET (WPF o Avalonia) | Electron |
|---|---|---|---|
| Tamaño del instalador | Pocos MB: WebView2 viene con Windows 10 (abril 2018+) y 11; el instalador por defecto solo descarga el runtime si falta (0 MB extra) [1] | Avalonia con Native AOT: ~18 MB según una plantilla de la comunidad, sin medida oficial [3] | 100–150 MB para una app mínima; casi todo es Chromium [4] |
| Overlay | `transparent` al crear la ventana, `set_always_on_top` y `set_ignore_cursor_events` en la API de Tauri 2.12 [2]. Sin probar: WebView2 transparente ha tenido fallos de pintado | WPF: lo más probado en Windows. Avalonia: transparencia completa solo en Windows; para dejar pasar clics hay que poner `Background="{x:Null}"` [3] | `setIgnoreMouseEvents` y ventanas transparentes, muy usados |
| Interfaz | HTML/TS: se puede compartir con la web (P-003) | XAML; nada se comparte con la web | HTML/TS: se comparte con la web |
| Lógica (parser, leaderboard) | Rust: tipos estrictos, binario nativo, tests con `cargo test` | C#: tipos estrictos, `dotnet test` | TS/Node |
| Autoactualización | Plugin `updater` oficial, con firma propia de las actualizaciones | Velopack u otro (dependencia externa) | `electron-updater` |
| Licencias para SignPath | MIT/Apache-2.0; WebView2 es librería del sistema | .NET y Avalonia MIT | Chromium y Electron BSD/MIT |
| Herramientas en el equipo | `cargo` 1.96 y Node 24 instalados | Sin SDK de .NET | Node 24 instalado |

SignPath Foundation pide que todos los componentes tengan licencia OSI, permite librerías del sistema, exige una build automatizada y verificable desde el repo (CI) y, para ejecutables, "cierta reputación verificable" del proyecto [5]. Las tres opciones cumplen lo de licencias; la reputación pesa igual en todas.

## Recomendación

**Tauri 2 (lógica en Rust, interfaz en TypeScript).**

- Instalador pequeño y autoactualización oficial con firma, que encaja con T-105.
- La interfaz en TS se puede reutilizar en la web (P-003) para el visor de partidas y los recaps (F2), en vez de hacerla dos veces.
- El parser y el cliente del leaderboard son lógica pura, fácil de portar a Rust con tests a partir de los prototipos y sus tests sintéticos.
- Rust y Node ya están instalados; .NET no.

**Riesgo principal: el overlay.** La API existe, pero no está probada en este proyecto. Antes de cerrar P-002 conviene una prueba corta: ventana Tauri transparente, siempre encima y que deje pasar los clics, encima de Hearthstone en ventana y en pantalla completa sin bordes. Si falla, el plan B es WPF solo para el overlay o para toda la app.

Electron queda descartado salvo que Tauri falle: hace lo mismo con un instalador 20–50 veces más grande.

## Siguientes pasos propuestos

1. El usuario decide P-002 (o pide antes la prueba del overlay).
2. Si sale Tauri: prueba del overlay (no toca el juego ni sus archivos; solo una ventana encima), y luego T-101 con el parser portado a Rust y los tests sintéticos de `tests/` como referencia.
3. Las dependencias nuevas (crates de Tauri, paquetes npm) se fijan con lockfile y se anotan en `PROVENANCE.md` (R2: `/security-review`).

## Fuentes

1. Tauri, [Windows Installer](https://v2.tauri.app/distribute/windows-installer/) (modos de WebView2 y tamaño que añaden).
2. Tauri, [`Window` en docs.rs](https://docs.rs/tauri/latest/tauri/window/struct.Window.html) (versión 2.12.2).
3. Avalonia, [How to: Work with Windows](https://docs.avaloniaui.net/docs/how-to/window-how-to) y [Native AOT](https://docs.avaloniaui.net/docs/deployment/native-aot); plantilla [AvaloniaAOT](https://github.com/lixinyang123/AvaloniaAOT) (~18 MB, cifra de la comunidad).
4. [Electron Packager](https://packages.electronjs.org/packager) (el binario precompilado marca el mínimo) y medidas de terceros de 115–151 MB; sin cifra oficial.
5. SignPath Foundation, [condiciones](https://signpath.org/terms).

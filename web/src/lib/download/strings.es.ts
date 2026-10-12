// Spanish (Spain) for lib/download/strings.ts (T-558): the same keys and placeholders.
import type { DlKey } from "./strings.ts";

export const ES: Record<DlKey, string> = {
  "page.kicker": "Windows · beta",
  "page.title": "Descargar Tavern Ledger",
  "page.lead":
    "Una aplicación gratuita para Windows que guarda todas las partidas de Campos de Batalla que juegas. Es una versión temprana para probadores beta: puede tener fallos.",

  "get.title": "El instalador",
  "fact.version": "Versión",
  "fact.version.value": "{version} (beta)",
  "fact.windows": "Funciona en",
  "fact.windows.value": "Windows 10 y 11, 64 bits",
  "fact.size": "Tamaño",
  "fact.size.value": "{size} MB",
  "fact.date": "Publicada",
  "action.download": "Descargar",
  "action.download.file": "Archivo: {file}",
  "action.github": "Todas las versiones en GitHub",

  "beta.title": "Una versión temprana, y aún sin firmar",
  "beta.body":
    "Windows te avisará al abrir el instalador (paso 2). Es normal por ahora. Si algo falla, cuéntalo con el botón «Informar de un problema» de la aplicación o en GitHub.",

  "steps.title": "Instálala en cuatro pasos",
  "step1.title": "Descarga el instalador",
  "step1.text":
    "Pulsa el botón dorado. Tu navegador puede pedirte que confirmes que quieres conservar un archivo que no conoce; conserva el archivo.",
  "step2.title": "Pasa el aviso de Windows",
  "step2.text":
    "Puede que Windows diga «Windows protegió su PC». Lo muestra porque el instalador aún no tiene certificado de firma de código y lo han descargado pocas personas. Pulsa Más información, comprueba que el nombre del archivo coincide y pulsa Ejecutar de todas formas. Hazlo solo con el archivo de esta página o de las versiones del proyecto en GitHub.",
  "step3.title": "Instálala",
  "step3.text":
    "Sigue el instalador. Se instala solo para tu usuario de Windows, así que no pide permisos de administrador, y añade un acceso directo en el menú Inicio. Si falta Microsoft Edge WebView2, el instalador lo obtiene de Microsoft (hace falta internet).",
  "step4.title": "Ábrela y juega",
  "step4.text": "Abre Tavern Ledger desde el menú Inicio. Guarda tus partidas de Campos de Batalla mientras juegas, y funciona sin cuenta.",
  "step4.account":
    "¿Quieres tus partidas también en la web? Inicia sesión allí y activa la subida en los Ajustes de la aplicación. Es opcional.",
  "step4.closed": "Las cuentas de la web aún no están abiertas; la aplicación no necesita una.",

  "check.title": "Comprobar el archivo",
  "check.body":
    "El instalador no está firmado, así que puedes comprobar que es el archivo que publicamos. En PowerShell, en la carpeta donde lo guardaste, ejecuta el comando de abajo; el resultado debe ser igual que este SHA-256:",
  "check.command": "Get-FileHash \"{file}\"",
  "reads.title": "Qué lee la aplicación",
  "reads.1": "Los archivos de registro del juego en tu PC, para guardar tus partidas de Campos de Batalla.",
  "reads.2": "Para el overlay, dónde está tu puntero sobre la ventana del juego.",
  "reads.3":
    "Solo si lo activas en Ajustes (está desactivado hasta que lo hagas): dos cosas que el juego ya te muestra, las razas de la partida y tu propia puntuación, leídas de la memoria del juego sin escribir en ella. Esto va en contra de la letra de las normas de Blizzard, y la aplicación te explica el riesgo antes de que elijas.",
  "reads.4": "Nunca cambia el juego, sus archivos ni su memoria, y nunca juega por ti.",
  "reads.data":
    "Tus partidas se quedan en tu PC, en %APPDATA%\\TavernLedger. No se sube nada a menos que inicies sesión y actives la subida.",
  "reads.privacy": "Política de privacidad",
  "update.title": "Actualizar",
  "update.body":
    "Aún no hay actualizaciones automáticas. Para actualizar, descarga el instalador nuevo de esta página y ejecútalo: sustituye la aplicación y conserva tus partidas y ajustes.",
  "remove.title": "Desinstalar",
  "remove.body":
    "Quítala en Configuración de Windows, en Aplicaciones. Tus partidas y ajustes de %APPDATA%\\TavernLedger se conservan, así que al reinstalar los encuentra; borra esa carpeta a mano si quieres que desaparezcan.",

  "source.body": "Código abierto con licencia AGPL-3.0. Puedes leer el código, o construir tú mismo el mismo instalador, en",
  "source.link": "GitHub",
};

// Spanish (Spain) for lib/download/strings.ts (T-558): the same keys and placeholders.
import type { DlKey } from "./strings.ts";

export const ES: Record<DlKey, string> = {
  "page.kicker": "Windows · beta",
  "page.title": "Descargar Tavern Ledger",
  "page.lead":
    "Una aplicación gratuita para Windows que guarda todas tus partidas de Campos de Batalla. Beta temprana: puede tener fallos.",

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

  "beta.line":
    "Versión temprana y sin firmar: Windows te avisará (paso 2). ¿Algún fallo? Usa «Informar de un problema» en la aplicación.",

  "steps.title": "Instálala en cuatro pasos",
  "step1.title": "Descarga el instalador",
  "step1.line": "Pulsa Descargar y conserva el archivo.",
  "step1.more":
    "Tu navegador puede pedirte que confirmes que quieres conservar un archivo que no conoce; conserva el archivo.",
  "step2.title": "Pasa el aviso de Windows",
  "step2.line": "Windows avisará: pulsa «Más información» → «Ejecutar de todas formas».",
  "step2.more":
    "Puede que Windows diga «Windows protegió su PC». Lo muestra porque el instalador aún no está firmado y lo han descargado pocas personas. Comprueba que el nombre del archivo coincide y hazlo solo con el archivo de esta página o de las versiones del proyecto en GitHub.",
  "step3.title": "Instálala",
  "step3.line": "Sigue el instalador; no pide permisos de administrador.",
  "step3.more":
    "Se instala solo para tu usuario de Windows y añade un acceso directo en el menú Inicio. Si falta Microsoft Edge WebView2, el instalador lo obtiene de Microsoft (hace falta internet).",
  "step4.title": "Ábrela y juega",
  "step4.line": "Abre Tavern Ledger desde el menú Inicio y juega.",
  "step4.more":
    "Guarda tus partidas de Campos de Batalla mientras juegas, y funciona sin cuenta.",
  "step4.account":
    "¿Quieres tus partidas también en la web? Crea una cuenta en la web, abre los Ajustes de la aplicación, busca «Subir a la web», pide el enlace de inicio de sesión y activa la subida. Es opcional.",
  "step4.closed": "Las cuentas de la web aún no están abiertas; la aplicación no necesita una.",

  "extras.title": "Conviene saber",
  "check.title": "Comprobar el archivo",
  "check.short": "El instalador no está firmado, así que puedes comprobar que es el archivo que publicamos.",
  "check.body": "En PowerShell, en la carpeta donde lo guardaste, ejecuta el comando de abajo. El resultado debe ser igual que este SHA-256 (PowerShell lo escribe en mayúsculas; da igual):",
  "check.command": "Get-FileHash \"{file}\"",
  "check.sums": "Esta versión también da el SHA-256 de cada uno de sus archivos, firmado con la clave minisign del proyecto:",
  "check.sums.list": "la lista (SHA256SUMS)",
  "check.sums.signature": "su firma",
  "reads.title": "Qué lee la aplicación",
  "reads.short": "Lee los archivos de registro del juego en tu PC, y nunca cambia el juego.",
  "reads.1": "Los archivos de registro del juego en tu PC, para guardar tus partidas de Campos de Batalla.",
  "reads.2": "Para el overlay, dónde está tu puntero sobre la ventana del juego.",
  "reads.3":
    "Solo si lo activas en Ajustes (está desactivado hasta que lo hagas): dos cosas que el juego ya te muestra, las razas de la partida y tu propia puntuación, leídas de la memoria del juego sin escribir en ella. Esto va en contra de la letra de las normas de Blizzard, y la aplicación te explica el riesgo antes de que elijas.",
  "reads.4": "Nunca cambia el juego, sus archivos ni su memoria, y nunca juega por ti.",
  "reads.data":
    "Tus partidas se quedan en tu PC, en %APPDATA%\\TavernLedger. No se sube nada a menos que inicies sesión y actives la subida.",
  "reads.privacy": "Política de privacidad",
  "update.title": "Actualizar",
  "update.short": "Aún no hay actualizaciones automáticas: descarga el instalador nuevo de esta página y ejecútalo.",
  "update.body": "Sustituye la aplicación y conserva tus partidas y ajustes.",
  "remove.title": "Desinstalar",
  "remove.short": "Quítala en Configuración de Windows, en Aplicaciones. Tus partidas se conservan.",
  "remove.body":
    "Tus partidas y ajustes de %APPDATA%\\TavernLedger se quedan, así que al reinstalar los encuentra. Borra esa carpeta a mano si quieres que desaparezcan.",

  "source.body": "Código abierto con licencia AGPL-3.0. Puedes leer el código, o construir tú mismo el mismo instalador, en",
  "source.link": "GitHub",
};

// Words of the updates setting and the main window's update notice (T-105b).
// Each row is [key, English, Spanish]. Prefix: updates.
"use strict";

(() => {
  const rows = [
    ["updates.title", "Updates", "Actualizaciones"],
    ["updates.current", "This app: version {version}", "Esta app: versión {version}"],
    ["updates.auto", "Check for updates automatically", "Buscar actualizaciones automáticamente"],
    ["updates.autoOff", "Check for updates automatically (not active in this version)", "Buscar actualizaciones automáticamente (no activo en esta versión)"],
    ["updates.checkNow", "Check now", "Buscar ahora"],
    ["updates.note", "Updates come from the project's GitHub releases. The app checks when it starts and every 6 hours, downloads a new version in the background and keeps it only if it carries the project's signature. It installs only when you say so, never during a game. Like any download, checking tells GitHub your IP address.", "Las actualizaciones vienen de las versiones del proyecto en GitHub. La app comprueba al arrancar y cada 6 horas, descarga la versión nueva en segundo plano y solo la guarda si lleva la firma del proyecto. Solo la instala cuando tú lo dices, nunca durante una partida. Como cualquier descarga, comprobar le dice a GitHub tu dirección IP."],
    ["updates.off", "Automatic updates aren't set up in this version of the app. Get new versions from tavernledger.net.", "Las actualizaciones automáticas no están activadas en esta versión de la app. Descarga las versiones nuevas en tavernledger.net."],
    ["updates.idle", "Not checked yet.", "Aún no se ha comprobado."],
    ["updates.checking", "Checking for updates…", "Buscando actualizaciones…"],
    ["updates.downloading", "Downloading version {version}…", "Descargando la versión {version}…"],
    ["updates.ready", "Version {version} is ready. Restart to update.", "La versión {version} está lista. Reinicia para actualizar."],
    ["updates.latest", "You have the latest version (checked {time}).", "Tienes la última versión (comprobado: {time})."],
    ["updates.installing", "Installing version {version}: the app closes and opens again.", "Instalando la versión {version}: la app se cierra y se vuelve a abrir."],
    ["updates.failed.network", "Couldn't reach GitHub to check for updates ({time}). The app tries again later.", "No se ha podido contactar con GitHub para buscar actualizaciones ({time}). La app lo vuelve a intentar más tarde."],
    ["updates.failed.not_found", "No update information was found ({time}). The app tries again later.", "No se ha encontrado información de actualización ({time}). La app lo vuelve a intentar más tarde."],
    ["updates.failed.signature", "An update was found but refused: it isn't signed by the project's key ({time}). Nothing was installed.", "Se ha encontrado una actualización, pero se ha rechazado: no lleva la firma del proyecto ({time}). No se ha instalado nada."],
    ["updates.failed.invalid", "The update information didn't look right, so nothing was downloaded ({time}).", "La información de la actualización no era correcta, así que no se ha descargado nada ({time})."],
    ["updates.failed.install", "The update couldn't be installed. Try again, or get it from tavernledger.net.", "No se ha podido instalar la actualización. Vuelve a intentarlo o descárgala en tavernledger.net."],
    ["updates.restart", "Restart now", "Reiniciar ahora"],
    ["updates.later", "Later", "Más tarde"],
    ["updates.notice", "Update", "Actualización"],
    ["updates.problem", "Couldn't do that: {error}", "No se ha podido hacer: {error}"],
  ];
  // Sentences the app itself sends (src/updates.rs).
  const backend = {
    "Finish your game first: updating restarts the app.": "Termina la partida primero: actualizar reinicia la app.",
    "No update is ready to install.": "No hay ninguna actualización lista para instalar.",
    "The update could not be installed.": "No se ha podido instalar la actualización.",
    "Updates are not set up in this build.": "Las actualizaciones no están activadas en esta versión.",
    "Checking for updates stopped. Restart the app.": "La búsqueda de actualizaciones se ha detenido. Reinicia la app.",
    "The update setting could not be saved; it will be lost after a restart.": "No se ha podido guardar el ajuste de actualizaciones; se perderá al reiniciar.",
    "The update setting could not be read; automatic checks are on.": "No se ha podido leer el ajuste de actualizaciones; la búsqueda automática está activada.",
  };

  const table = { en: {}, es: {}, backend: { es: backend } };
  rows.forEach(([key, en, es]) => {
    table.en[key] = en;
    table.es[key] = es;
  });

  const i18n = typeof module !== "undefined" && module.exports ? require("../i18n.js") : window.TLI18n;
  i18n.define(table);
  if (typeof module !== "undefined" && module.exports) module.exports = table;
})();

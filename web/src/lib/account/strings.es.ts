// Spanish (Spain) for lib/account/strings.ts (T-508): the same keys and placeholders.
import type { AccountKey } from "./strings.ts";

export const ES: Record<AccountKey, string> = {
  // Sign in (components/SignIn.astro, scripts/signin.ts).
  "signin.title": "Iniciar sesión",
  "signin.meta.before": "Correo electrónico y contraseña. La web",
  "signin.meta.link": "aún no está abierta al público",
  "signin.meta.after": ": las cuentas son solo para pruebas.",
  "signin.have": "Ya tengo una cuenta",
  "signin.email": "Correo electrónico",
  "signin.password": "Contraseña",
  "signin.submit": "Iniciar sesión",
  "signin.forgot": "¿Has olvidado la contraseña?",
  "signin.create": "Crear una cuenta",
  "signin.hint.length": "Al menos {min} caracteres.",
  "signin.consent.before": "Al crear una cuenta aceptas los",
  "signin.consent.terms": "términos",
  "signin.consent.middle": "y confirmas que tienes 16 años o más. La",
  "signin.consent.privacy": "política de privacidad",
  "signin.consent.after": "explica qué hacemos con tus datos.",
  "signin.createSubmit": "Crear cuenta",
  "signin.closed": "El registro aún no está abierto.",
  "signin.signingIn": "Iniciando sesión…",
  "signin.creating": "Creando la cuenta…",

  // Forgot password (components/ForgotPassword.astro, scripts/forgot.ts).
  "forgot.title": "Contraseña olvidada",
  "forgot.meta":
    "Escribe el correo electrónico de tu cuenta. Te enviamos un enlace para elegir una contraseña nueva; ábrelo en este navegador.",
  "forgot.heading": "Recibir un enlace de restablecimiento",
  "forgot.submit": "Enviar el enlace",
  "forgot.back": "Volver a iniciar sesión",
  "forgot.sending": "Enviando…",

  // Account (components/Account.astro, scripts/account.ts).
  "account.title": "Tu cuenta",
  "account.loading": "Cargando…",
  "account.signedOut.before": "No has iniciado sesión.",
  "account.signedOut.link": "Inicia sesión",
  "account.signedOut.after": "para ver tu cuenta.",
  "account.setPassword": "Establecer una contraseña nueva",
  "account.newPassword": "Contraseña nueva",
  "account.repeat": "Repítela",
  "account.savePassword": "Guardar la contraseña nueva",
  "account.deleted": "Tu cuenta y todos sus datos se han borrado. Gracias por probar Tavern Ledger.",
  "account.signedInAs.before": "Sesión iniciada como",
  "account.profile": "Perfil",
  "account.displayName": "Nombre visible",
  "account.displayName.hint":
    "Hasta 32 letras, dígitos, espacios, _ . o -. No es tu BattleTag. Se rechazan las palabras ofensivas y los nombres que parezcan oficiales.",
  "account.public": "Perfil público",
  "account.public.hint.before":
    "Desactivado por defecto. Si lo activas, cualquiera puede encontrar tu perfil y ver tu nombre visible y todas las partidas que subiste, al completo: héroe, puesto, modo, fecha, tribus ofrecidas y los tableros que viste. Con un nombre visible y suficientes partidas, también apareces en las",
  "account.public.hint.link": "clasificaciones públicas",
  "account.public.hint.after":
    ". Nunca tu puntuación ni los nombres de otros jugadores. Al desactivarlo, dejas de aparecer en todas partes en cuanto guardes.",
  "account.save": "Guardar",
  "account.publicPage": "Tu página pública:",
  "account.data": "Tus datos",
  "account.data.body":
    "Descarga todo lo que esta web guarda sobre ti en un solo archivo JSON: cuenta, historial de inicios de sesión, perfil y ajustes, todas las partidas y los propios archivos del juego.",
  "account.download": "Descargar mis datos",
  "account.community": "Estadísticas de la comunidad",
  "account.community.label": "Incluir mis partidas en las estadísticas de héroes de la comunidad",
  "account.community.hint.before":
    "Desactivado por defecto y independiente de tu perfil público. Si lo activas, tus partidas terminadas cuentan para las",
  "account.community.hint.link": "estadísticas de héroes",
  "account.community.hint.after":
    ", que se muestran solo como totales redondeados de al menos 5 jugadores y nunca con tu nombre; al desactivarlo, tus partidas se retiran en la siguiente actualización diaria.",
  "account.community.how": "Cómo funciona",
  "account.delete": "Borrar cuenta",
  "account.delete.body":
    "Esto borra tu cuenta, tu perfil, todas las partidas y archivos subidos y tu historial de inicios de sesión. No se puede deshacer. Descarga antes tus datos si quieres conservarlos.",
  "account.delete.type.before": "Escribe",
  "account.delete.type.after": "para confirmar",
  "account.delete.phrase": "borrar mi cuenta",
  "account.reauth": "Por tu seguridad, vuelve a escribir tu contraseña",
  "account.reauth.hint": "Para borrar hace falta haber iniciado sesión en los últimos 10 minutos.",
  "account.delete.submit": "Borrar mi cuenta",
  "account.signOut": "Cerrar sesión",
  "account.profile.loadFailed": "No se pudo cargar tu perfil. Recarga la página para intentarlo de nuevo.",
  "account.saving": "Guardando…",
  "account.saved": "Guardado.",
  "account.setting.loadFailed": "No se pudo cargar este ajuste. Recarga la página para intentarlo de nuevo.",
  "account.community.savedOn": "Guardado. Tus partidas cuentan a partir de la siguiente actualización diaria.",
  "account.community.savedOff": "Guardado. Tus partidas quedan fuera a partir de la siguiente actualización diaria.",
  "account.community.unchanged": "{error} Tu ajuste no ha cambiado.",
  "account.export.preparing": "Preparando tu archivo…",
  "account.export.done": "Descargado.",
  "account.export.failed": "La exportación ha fallado. Inténtalo de nuevo en un momento.",
  "account.delete.typeExactly": "Escribe la frase exactamente para confirmar.",
  "account.delete.checking": "Comprobando tu contraseña…",
  "account.delete.deleting": "Borrando…",
  "account.delete.needPassword": "Escribe tu contraseña y pulsa de nuevo Borrar mi cuenta.",
  "account.password.saved": "Tu contraseña nueva se ha guardado.",
  "account.loadFailed": "No se pudo cargar tu cuenta. Recarga la página para intentarlo de nuevo.",

  // Rules and server answers (lib/auth.ts).
  "auth.checkEmail":
    "Revisa tu bandeja de entrada: si este correo se puede usar, te llegará un enlace de confirmación. Ábrelo en este navegador para terminar.",
  "auth.wrong": "Correo electrónico o contraseña incorrectos.",
  "auth.rateLimited": "Demasiados intentos. Espera unos minutos e inténtalo de nuevo.",
  "auth.unreachable": "No se pudo conectar con el servidor. Comprueba tu conexión e inténtalo de nuevo.",
  "auth.generic": "Algo ha ido mal. Inténtalo de nuevo en un momento.",
  "auth.email": "Escribe una dirección de correo electrónico válida.",
  "auth.passwordLength": "La contraseña necesita al menos {min} caracteres.",
  "auth.confirmFirst": "Confirma antes tu correo electrónico: abre el enlace que te enviamos.",
  "auth.stronger": "Elige una contraseña más segura (al menos {min} caracteres).",
  "auth.signupClosed": "El registro aún no está abierto.",
  "auth.resetSent":
    "Revisa tu bandeja de entrada: si este correo tiene una cuenta, te llegará un enlace para elegir una contraseña nueva. Ábrelo en este navegador.",
  "auth.notSame": "Las dos contraseñas no coinciden.",
  "auth.samePassword": "Elige una contraseña distinta de la actual.",
  "auth.resetExpired": "Este restablecimiento ha caducado. Pide un enlace nuevo con «¿Has olvidado la contraseña?» en la página de inicio de sesión.",

  // Links from emails (lib/authLink.ts).
  "link.useForgot": "Para elegir una contraseña, usa «¿Has olvidado la contraseña?» en la página de inicio de sesión.",
  "link.confirmed": "Tu correo electrónico está confirmado.",
  "link.confirmedSignIn": "Tu correo electrónico está confirmado. Inicia sesión para continuar.",
  "link.magic":
    "Los enlaces de inicio de sesión son para la aplicación Tavern Ledger. Aquí, inicia sesión con tu contraseña o usa «¿Has olvidado la contraseña?» en la página de inicio de sesión para establecer una.",
  "link.recovery":
    "Este enlace de restablecimiento no se puede usar aquí. Pide uno nuevo con «¿Has olvidado la contraseña?» en la página de inicio de sesión.",
  "link.newEmail": "Tu nueva dirección de correo electrónico está confirmada.",
  "link.newEmailSignIn": "Tu nueva dirección de correo electrónico está confirmada. Inicia sesión para continuar.",
  "link.notice": "Enlace aceptado. Ahora abre el enlace que enviamos a tu otra dirección de correo electrónico para terminar el cambio.",
  "link.expired":
    "Este enlace ha caducado o ya se usó (cada correo nuevo sustituye al enlace anterior). Si estabas confirmando tu correo electrónico, prueba a iniciar sesión: puede que ya esté confirmado. Si no, pide un enlace nuevo.",
  "link.otherBrowser":
    "Este enlace se abrió en un navegador distinto de aquel en el que lo pediste. Si estabas confirmando tu correo electrónico, ya está confirmado: inicia sesión. Si estabas restableciendo tu contraseña, abre el enlace en el navegador donde lo pediste o pide uno nuevo.",
  "link.unreachable": "No se pudo conectar con el servidor. Comprueba tu conexión y recarga la página.",
  "link.failed": "Este enlace no ha funcionado. Inicia sesión o pide un enlace nuevo.",

  // Display name and saves (lib/profile.ts).
  "profile.maxLength": "Hasta {max} caracteres.",
  "profile.chars": "Usa solo letras, dígitos, espacios, _ . o -.",
  "profile.notAllowed": "Este nombre no está permitido. Elige otro: los nombres se muestran en las páginas públicas y las clasificaciones.",
  "profile.charsRange": "Usa de 1 a 32 letras, dígitos, espacios, _ . o - únicamente.",
  "profile.sessionEnded": "Tu sesión ha terminado. Vuelve a iniciar sesión.",
  "profile.saveFailed": "No se pudo guardar. Inténtalo de nuevo en un momento.",

  // Account deletion (lib/deleteAccount.ts).
  "delete.unexpected": "El servidor dio una respuesta inesperada. Inténtalo de nuevo.",
  "delete.unreachable": "No se pudo conectar con el servidor. Todavía no se ha borrado nada; inténtalo de nuevo.",
  "delete.unfinished": "El borrado no ha terminado. Inténtalo de nuevo: continúa donde se quedó.",
  "delete.unfinishedStep": "El borrado no ha terminado (paso: {step}). Inténtalo de nuevo: continúa donde se quedó.",
  "delete.refused": "El servidor rechazó la solicitud. Recarga la página e inténtalo de nuevo.",
};

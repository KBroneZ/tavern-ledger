# Política de firma de código

*Traducción del original en inglés. Si esta versión y la [versión en inglés](https://tavernledger.net/code-signing/) dicen cosas distintas, prevalece la inglesa.*

Última actualización: 17 de octubre de 2026

<span lang="en">Free code signing provided by <a href="https://signpath.io">SignPath.io</a>, certificate by <a href="https://signpath.org">SignPath Foundation</a>.</span>

El instalador de Tavern Ledger para Windows se firma para que Windows muestre quién lo publica y avise solo si el archivo ha sido modificado. Esta página explica qué se firma, quién decide y qué hace con tus datos el programa firmado.

## Qué se firma

El instalador de Windows (`TavernLedger_X.Y.Z_x64-setup.exe`) de cada versión. Se construye a partir del código fuente de este repositorio con GitHub Actions, en los servidores de GitHub y nunca en el PC de un desarrollador, desde una etiqueta de versión de la rama `main` (el flujo es `.github/workflows/release.yml`). SignPath comprueba que el archivo sale de verdad de esa construcción antes de firmarlo.

Cada versión se aprueba a mano antes de firmarse. No se firma nada más con este certificado, y solo se firma software construido a partir del código fuente de este proyecto.

El editor que Windows muestra para un instalador firmado es SignPath Foundation, porque el certificado es suyo.

## Roles del equipo

- **Autores y revisores:** [KBroneZ](https://github.com/KBroneZ), el responsable del proyecto. Un cambio de cualquier otra persona lo revisa el responsable antes de incorporarlo.
- **Aprobadores:** [KBroneZ](https://github.com/KBroneZ). Cada solicitud de firma se aprueba a mano.

Todas las personas con estos roles usan autenticación de varios factores en GitHub y en SignPath.

## Política de privacidad

Esa política explica qué hacen el programa y la web con tus datos, incluidas todas las direcciones a las que el programa se conecta: [Política de privacidad](https://tavernledger.net/es/privacy/).

## Contacto

Para avisar de un problema con un archivo firmado, sigue los pasos de [SECURITY.md](https://github.com/KBroneZ/tavern-ledger/blob/main/SECURITY.md) o escribe a contact@tavernledger.net.

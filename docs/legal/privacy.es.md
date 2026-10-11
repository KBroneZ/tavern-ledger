# Política de privacidad

*Traducción del original en inglés. Si esta versión y la [versión en inglés](https://tavernledger.net/privacy/) dicen cosas distintas, prevalece la inglesa.*

Última actualización: 12 de octubre de 2026

**Qué cambió el 12 de octubre de 2026:** con tu perfil público, ahora apareces en una clasificación desde tu primera partida terminada en ese modo y periodo, no a partir de 20 (apartado 4, «Perfiles públicos y clasificaciones»); un número de pocas partidas lleva la marca «pocas partidas».

**Qué cambió el 11 de octubre de 2026:** clasificaciones públicas (apartado 4, «Perfiles públicos y clasificaciones»): qué muestran, quién aparece en ellas, cómo salir de ellas y durante cuánto tiempo se conservan los números; la comprobación de los nombres visibles con una lista corta de palabras; los números de las clasificaciones en la descarga (apartado 7). También nuevo ese día: imágenes de cartas y héroes cargadas desde HearthstoneJSON (apartados 3 y 5); estadísticas de héroes de la comunidad (apartado 4, «Estadísticas de héroes de la comunidad»), que cuentan tus partidas solo si las activas, con el interruptor en la descarga (apartado 7) y su conservación (apartado 6).

Tavern Ledger es un rastreador gratuito y de código abierto de Campos de Batalla de Hearthstone: una aplicación para Windows que lee los archivos de registro del juego en tu PC y un sitio web donde puedes guardar tus partidas y, si quieres, compartirlas. Esta política explica qué datos personales tratamos, para qué, adónde van y qué puedes hacer al respecto.

Proyecto no oficial de fans. Sin afiliación con Blizzard Entertainment ni respaldo de esta.

## 1. Quién es el responsable

El responsable del tratamiento es Andrew Rodrigo, Bulgaria, una persona física que lleva este proyecto en su tiempo libre.

Contacto para cualquier asunto sobre tus datos: privacy@tavernledger.net.

## 2. La aplicación de escritorio guarda tus datos en tu PC

La aplicación lee los archivos de registro del juego y guarda un resumen de cada partida de Campos de Batalla en `%APPDATA%\TavernLedger\games.jsonl`: la hora de inicio de la sesión, el modo, la versión del juego, tu héroe y el héroe de tu compañero de Dúos, los héroes de la partida por número de jugador, puestos y vida, tableros (como ids de carta), nombres de cartas en el idioma de tu juego, tribus ofrecidas en la taberna, tu propia tienda en cada turno (las cartas ofrecidas, compradas y vendidas, refrescos, congelaciones, oro y nivel de taberna) y las acciones tuyas que el juego registró, con el tiempo transcurrido desde el inicio de la partida (para calcular tus acciones por minuto; junto con la hora de inicio de la partida, esto indica aproximadamente cuándo ocurrió cada acción), y la versión del analizador que leyó la partida. Los registros contienen los nombres (BattleTags) de los jugadores de tu partida; la aplicación no los guarda.

**Lectura del juego (opcional, desactivada por defecto).** Si activas «Leer del juego las tribus de la partida y tu puntuación», la aplicación también lee, en modo solo lectura, dos valores que el juego te muestra mientras se ejecuta: las cinco tribus de la partida en la que estás y tu propia puntuación de Campos de Batalla (Solo y Dúos). No lee nada más del juego: ni nombres de otros jugadores, ni BattleTags, ni ids de cuenta, ni puntuaciones. Estos valores permanecen en la memoria de la aplicación en tu PC: no se guardan con tus partidas, no se incluyen en el registro de la partida y nunca se suben. Al desactivar el interruptor, los olvida.

Mientras no actives la subida, la aplicación no envía nada por internet y nunca recibimos este archivo. Para exportarlo, cópialo; para borrarlo, borra la carpeta `TavernLedger`.

**Subida a tu cuenta (opcional, desactivada por defecto).** Puedes iniciar sesión desde la aplicación con un enlace de un solo uso enviado a tu correo electrónico y abierto en tu navegador; la aplicación nunca te pide la contraseña. Guarda el inicio de sesión (un token de renovación) en el Administrador de credenciales de Windows de tu PC, y una lista de las partidas que subió junto al historial (`uploads.jsonl`). Solo cuando activas «Subir partidas» envía el registro de cada partida terminada (el resumen anterior, sin nombres de jugadores) a tu cuenta (apartado 4). Al cerrar sesión se termina la sesión en nuestro servidor, se elimina el inicio de sesión de tu PC y se desactiva la subida.

## 3. Cuando visitas el sitio web

Cuando cargas una página, nuestro proveedor de alojamiento (Cloudflare) y, para las páginas que hablan con nuestro servidor, nuestro proveedor de backend (Supabase) reciben tu dirección IP, el agente de usuario de tu navegador, la dirección solicitada y la hora, como en cualquier sitio web. Lo usan para entregar la página y proteger el servicio, y lo conservan en sus registros (apartado 6).

Base jurídica: nuestro interés legítimo en mantener un sitio web que funcione y esté protegido contra ataques (art. 6.1.f del RGPD).

**Imágenes de cartas.** Las páginas que muestran partidas (una partida, tus partidas, un perfil público, la lista de partidas y las clasificaciones) cargan imágenes de cartas y héroes directamente del servidor de imágenes de HearthstoneJSON (`art.hearthstonejson.com`), un servicio independiente gestionado por HearthSim, no un proveedor nuestro. Tu navegador le envía tu dirección IP, el agente de usuario de tu navegador y la dirección de cada imagen, que nombra solo la carta: nuestras páginas no envían la cabecera Referer, así que no sabe qué página, cuenta o partida estás viendo. HearthSim decide qué registra; consulta hearthstonejson.com. Base jurídica: nuestro interés legítimo en mostrar las partidas de forma legible (art. 6.1.f del RGPD). Para evitarlo, bloquea esa dirección en tu navegador: las páginas mostrarán entonces un marcador sencillo en su lugar.

No hay anuncios, ni analítica, ni rastreo.

## 4. Si creas una cuenta

| Datos | Para qué | Base jurídica |
|------|-----|-------------|
| Dirección de correo electrónico, contraseña (almacenada solo como hash), id de cuenta, fechas y horas de registro, de confirmación y del último inicio de sesión | Para crear tu cuenta y que inicies sesión | Contrato (art. 6.1.b): necesario para prestar la cuenta que has solicitado |
| Correos de la cuenta: tu dirección, el mensaje (un enlace de confirmación, de inicio de sesión o de restablecimiento de contraseña) y su estado de entrega | Para confirmar tu dirección y proteger tu cuenta | Contrato |
| Sesiones de inicio de sesión: horas, dirección IP y agente de usuario del navegador; tokens de renovación y de un solo uso | Para mantener tu sesión iniciada | Contrato |
| Registro de seguridad: altas de cuenta, inicios y cierres de sesión, solicitudes de restablecimiento de contraseña, cambios de contraseña y de correo, con hora y dirección IP, conservado en nuestra base de datos durante 90 días | Para detectar y detener el abuso de cuentas | Interés legítimo en mantener las cuentas seguras (art. 6.1.f) |
| Perfil: un nombre visible que eliges (opcional) y si tu perfil es público (desactivado por defecto) | Para mostrar tu perfil y ponerte en las clasificaciones públicas, si lo haces público | Contrato: solo se publica cuando lo activas, y puedes desactivarlo en cualquier momento |
| Elección sobre estadísticas de la comunidad: si activaste «Incluir mis partidas en las estadísticas de héroes de la comunidad» (desactivado por defecto), cuándo lo cambiaste por última vez y la versión de la política vigente entonces | Para contar tus partidas en las estadísticas de héroes de la comunidad solo si lo pediste | Consentimiento (art. 6.1.a): puedes retirarlo en cualquier momento desactivando el interruptor |
| Estadísticas de héroes de la comunidad: a partir de las partidas de las cuentas que activaron el interruptor, totales por héroe, modo y periodo, reconstruidos a diario, sin id de cuenta | Para mostrar las estadísticas de héroes de la comunidad | Consentimiento, como arriba |
| Números de las clasificaciones: para un perfil público con nombre visible, los números que ordena cada clasificación (calculados a partir de tus partidas subidas) por modo y periodo, con cuántas partidas respaldan cada uno | Para mostrar las clasificaciones públicas | Contrato, igual que para el perfil público |
| Tus partidas: una fila resumen por partida (fecha, modo, héroe, puesto, versión, tribus ofrecidas, versión del analizador) y el registro completo de la partida desde la aplicación (sin nombres de jugadores), si los subes | Para mostrar tu historial y tus estadísticas | Contrato |
| Horas de subida: cuándo hiciste cada subida, conservadas dos días | Para aplicar los límites de subida por hora y por día | Interés legítimo en mantener el servicio disponible para todos (art. 6.1.f) |
| Solicitudes rechazadas: un hash con sal de la dirección IP que envió una subida con un inicio de sesión no válido (la dirección en sí no se almacena, y la sal secreta se cambia cada día), y un recuento, conservados una hora como máximo | Para bloquear solicitudes abusivas | Interés legítimo en proteger el servicio |
| Copias de seguridad de todo lo anterior, sin sesiones de inicio de sesión activas | Para recuperarnos de fallos | Interés legítimo en no perder tus datos |

Para tener una cuenta necesitamos tu correo electrónico y una contraseña; sin ellos no podemos crearla. Todo lo demás es opcional: un nombre visible, subir partidas, un perfil público y las estadísticas de héroes de la comunidad.

**No** recogemos nombres de jugadores, BattleTags, cuentas de Battle.net, puntuaciones ni MMR. No tomamos ninguna decisión automatizada sobre ti que tenga efectos jurídicos o que te afecte significativamente de modo similar.

<span id="public-profiles-and-leaderboards"></span>

### Perfiles públicos y clasificaciones

**Perfil público.** Si lo activas, cualquier persona en internet puede ver tu nombre visible, leer completas todas las partidas que subiste, y enlazarlas o copiarlas. Al desactivarlo deja de haber acceso a través de nuestro sitio; las copias que otros ya hayan hecho quedan fuera de nuestro alcance.

**Clasificaciones** (`/leaderboards/`). Mientras tu perfil sea público y tenga un nombre visible, también puedes aparecer en las clasificaciones públicas, que ordenan a los jugadores según sus partidas subidas, por modo (Solo, Dúos) y periodo (los últimos 7 días, los últimos 30 días, desde siempre). Una clasificación muestra tu nombre visible con un enlace a tu perfil público, tu posición en la clasificación, el número por el que se ordena (por ejemplo tu puesto medio, tu tasa de top 4 o top 2, tu tasa de victorias, partidas jugadas, tu racha más larga en la mitad superior, puesto medio con un héroe y ese héroe, acciones por minuto, oro más allá de los ingresos, refrescos o triples por partida, tu tablero más grande o más esbirros dorados) y en cuántas partidas se basa. Apareces en una clasificación en cuanto tienes una partida terminada en ese modo y periodo que tenga ese número (una con el héroe para la clasificación por héroe); un número de menos de 20 partidas (10 con el héroe) lleva la marca «pocas partidas». Los números son autodeclarados: proceden de las partidas que subió tu aplicación; nuestro servidor comprueba el registro de cada partida y deja fuera las partidas con valores que ninguna partida real puede tener, pero no puede demostrar que una partida se jugara tal como se subió. Las clasificaciones nunca muestran tu correo electrónico, BattleTag, puntuación ni los nombres de otros jugadores. La página pide a los buscadores que no la indexen.

**Salir de las clasificaciones.** Desactiva «Perfil público» en la página de tu cuenta, o elimina tu nombre visible: desde el momento en que guardes, nuestro servidor deja de mostrarte en todas las clasificaciones, y la copia almacenada de los números de tus clasificaciones se borra a la vez (o, si justo entonces hay una reconstrucción en curso, la borra esa reconstrucción o la siguiente). Una página de clasificaciones que alguien ya tenga abierta conserva lo que cargó hasta que la recargue. Si vuelves a hacer público tu perfil, regresas en la siguiente reconstrucción; las reconstrucciones están programadas cada 10 minutos.

**Cuánto tiempo se conservan los números.** Las clasificaciones se reconstruyen a partir de las partidas actuales cada 10 minutos; cada reconstrucción sustituye a la anterior, así que no se conserva ningún historial de clasificaciones pasadas. Tus números desaparecen cuando haces tu perfil privado, eliminas tu nombre visible o borras tu cuenta, y de las copias de seguridad según el apartado 6.

**Nombres visibles.** Cuando estableces un nombre visible, nuestro servidor lo comprueba con una lista corta de palabras ofensivas y de nombres que parecerían oficiales (como «admin» o «Blizzard») y lo rechaza si coincide; la página de la cuenta te pide entonces otro. Un nombre establecido antes de que se añadiera una palabra a la lista queda oculto en las clasificaciones hasta que lo cambies. La comprobación solo compara palabras; no decide nada más sobre ti.

<span id="community-hero-stats"></span>

### Estadísticas de héroes de la comunidad

**Qué es.** La página de estadísticas de héroes (`/heroes/`) muestra, por modo (Solo, Dúos) y periodo (los últimos 30 días, desde siempre), cómo le va a cada héroe: aproximadamente cuántas partidas y jugadores hay detrás, con qué frecuencia se elige cuando se ofrece, su puesto medio, su tasa de top 4 (Solo) o top 2 (Dúos) y su tasa de victorias.

**De quién son las partidas.** Solo las partidas de las cuentas que activaron «Incluir mis partidas en las estadísticas de héroes de la comunidad» en la página de su cuenta. El interruptor está desactivado para todas las cuentas, incluidas las creadas antes de que existiera esta página, y ninguna actualización lo activa. Es independiente de tu perfil público: un perfil público no se cuenta salvo que actives el interruptor, y un perfil privado sí puede contarse. Guardamos si el interruptor está activado, cuándo lo cambiaste por última vez y qué versión de esta política estaba vigente entonces.

**Qué se usa.** De cada una de tus partidas subidas y terminadas que nuestro servidor haya resumido, hasta el día anterior a cada reconstrucción: el modo, el día en que se jugó, tu héroe, tu puesto final y, si la aplicación registró tu elección de héroe, los héroes que se te ofrecieron. Las partidas con valores que ninguna partida real puede tener se dejan fuera, como en las clasificaciones. Para contar jugadores distintos y aplicar las reglas de abajo, la reconstrucción usa internamente tu id de cuenta; no se almacena junto con los totales.

**Qué se muestra.** Solo totales por héroe, y solo para un héroe con al menos 50 partidas de al menos 5 jugadores, donde ningún jugador tenga más de la mitad de las partidas y ningún par de jugadores más de tres cuartas partes. La tasa de elección sigue la misma regla sobre las partidas en las que se ofreció el héroe. «Histórico» muestra un héroe solo si sus partidas anteriores a los últimos 30 días cumplen por sí solas la misma regla (o no hay ninguna), de modo que los dos periodos no se puedan restar para señalar unas pocas partidas. Los recuentos se muestran redondeados a decenas de partidas y a múltiplos de cinco jugadores («50+», «5+»), el puesto medio con un decimal y las tasas en porcentaje entero. No se muestra ningún nombre visible, cuenta, BattleTag, puntuación ni partida individual, y los totales almacenados no conservan ningún id de cuenta. La página pide a los buscadores que no la indexen.

**Qué sigue siendo posible.** Los totales se reconstruyen una vez al día a partir de días completos de partidas, así que cada cambio mezcla las partidas de todos de ese día. Aun así, con pocos jugadores, alguien que guarde la página cada día y ya sepa algo sobre ti (por ejemplo, que acabas de empezar o dejar de compartir, o las partidas de tu perfil público) podría hacer una estimación aproximada sobre tus resultados o sobre si compartes; el redondeo lo deja en una estimación, no en un resultado exacto. Cualquiera puede guardar lo que muestra la página; nosotros no conservamos ningún historial de totales pasados.

**Dejar de participar.** Desactiva el interruptor en la página de tu cuenta. Tus partidas se dejan fuera a partir de la siguiente reconstrucción diaria. Si alguna reconstrucción fallara, la página deja de mostrar números cuando la última tiene más de 26 horas, así que tus partidas nunca siguen en lo que se muestra más tiempo que eso. Borrar tu cuenta funciona del mismo modo. Una página que alguien ya tenga abierta conserva lo que cargó hasta que la recargue. Retirar tu consentimiento no cambia lo que se mostró antes.

## 5. Quién recibe tus datos

**Nuestros proveedores**, como encargados del tratamiento que actúan según nuestras instrucciones, cada uno con un contrato de encargado del tratamiento (DPA) que forma parte de sus condiciones:

| Proveedor | Qué hace | Dónde | Transferencias fuera de la UE |
|----------|--------------|-------|--------------------------|
| Supabase (entidad contratante Supabase Pte. Ltd., Singapur) | Base de datos, inicio de sesión, almacenamiento de archivos, funciones de servidor y sus registros | Fráncfort, Alemania (UE) | Los datos se almacenan en la UE, pero Supabase y sus subencargados pueden acceder a ellos desde otros países; cubierto por las Cláusulas Contractuales Tipo de la UE del DPA de Supabase, que forma parte de sus condiciones de servicio; Supabase también ofrece una evaluación de impacto de las transferencias. Subencargados: lista del 1 de junio de 2026 en supabase.com/legal/subprocessors. |
| Cloudflare, Inc. (EE. UU.) | Aloja las páginas del sitio web; reenvía el correo enviado a nuestras direcciones de contacto | Red mundial | Marco de Privacidad de Datos UE-EE. UU., con Cláusulas Contractuales Tipo como respaldo, en el DPA para clientes de Cloudflare (versión 6.4 del 3 de abril de 2026) |
| Lettermint B.V., Países Bajos | Envía los correos de la cuenta desde noreply@tavernledger.net (confirmación de registro, enlaces de inicio de sesión, cambio de correo y restablecimiento de contraseña). El seguimiento de aperturas y de clics está desactivado en nuestra cuenta, así que los enlaces te llegan sin modificar | Unión Europea | Ninguna para los datos de correo: el contrato de encargado del tratamiento de Lettermint (parte de sus condiciones) los mantiene en la UE y no permite ninguna transferencia fuera del EEE sin nuestra aprobación. Dos de sus subencargados, Slack (mensajería del personal) y Stripe (facturación), están fuera de la UE y no reciben los correos |

Puedes pedirnos una copia de las garantías de transferencia en la dirección de contacto indicada arriba.

**HearthSim**, el servicio independiente que está detrás de HearthstoneJSON, recibe las solicitudes de imágenes de cartas descritas en el apartado 3.

**Todo el mundo**, si haces público tu perfil: tu perfil público y, con un nombre visible y suficientes partidas, las clasificaciones (apartado 4). Si activas las estadísticas de héroes de la comunidad: totales por héroe que incluyen tus partidas sin nombrarte (apartado 4).

**Autoridades**, solo cuando la ley lo exija.

No vendemos ni alquilamos tus datos.

## 6. Cuánto tiempo los conservamos

- **Cuenta, perfil, partidas y sesiones:** hasta que borres tu cuenta (las sesiones también terminan cuando cierras sesión o caducan).
- **Números de las clasificaciones:** reconstruidos cada 10 minutos mientras tu perfil sea público; borrados cuando lo haces privado o borras tu cuenta.
- **Elección sobre estadísticas de la comunidad:** hasta que borres tu cuenta. **Estadísticas de héroes de la comunidad:** reconstruidas una vez al día; tus partidas salen de ellas en la primera reconstrucción posterior a desactivar el interruptor o borrar tu cuenta, y la página no muestra nada cuando la última reconstrucción tiene más de 26 horas.
- **Registro de seguridad:** 90 días, y después lo borra cada día nuestra tarea de limpieza; todo se borra antes si borras tu cuenta.
- **Copias de seguridad:** se hacen semanalmente en el propio ordenador del responsable, legibles solo por su cuenta de usuario, y se borran a los 35 días. Si borras tu cuenta, tus datos pueden permanecer en las copias de seguridad hasta 35 días. Si alguna vez restauramos una copia de seguridad, volvemos a borrar las cuentas eliminadas desde que se hizo.
- **Horas de subida:** dos días. **Contadores de solicitudes rechazadas:** una hora.
- **Registros de los proveedores** (solicitudes de página, solicitudes al servidor, correos enviados): los conservan Cloudflare, Supabase y el remitente de correo durante los periodos que establezcan sus servicios, y después se borran. No están vinculados a tu cuenta de un modo que podamos exportarlos o borrarlos uno por uno. Supabase conserva sus registros de API, base de datos e inicio de sesión durante un día en nuestro plan. Lettermint conserva cada correo que envió, con su registro de entrega, durante 28 días y después lo borra. Cloudflare no nos proporciona los registros de solicitudes del sitio web en nuestro plan y no publica un periodo fijo para ellos; según su política de privacidad, los conserva solo mientras los necesite para operar y proteger su red. Para el correo que envíes a nuestras direcciones de contacto, Cloudflare conserva un registro de entrega (remitente, destinatario, asunto, hora y resultado) durante 31 días; el mensaje en sí se reenvía a nuestro buzón.

## 7. Tus derechos

Puedes, en cualquier momento:

- **Descargar tus datos:** «Descargar mis datos» en la página de tu cuenta te da un archivo JSON con los datos de tu cuenta (no el hash de tu contraseña ni los tokens de inicio de sesión), sesiones de inicio de sesión, registro de seguridad, perfil, tu elección sobre estadísticas de la comunidad, filas de partidas, los números de tus clasificaciones y archivos de partidas. Es legible por máquina, así que puedes llevártelo a otro sitio (portabilidad de los datos).
- **Solicitar el acceso** a cualquier otra cosa que tengamos sobre ti, como lo que pueda haber en los registros de los proveedores o en las copias de seguridad, escribiéndonos.
- **Rectificar** tu nombre visible en la página de la cuenta; para cualquier otra cosa, escríbenos.
- **Suprimir** tu cuenta: «Borrar cuenta» en la página de tu cuenta elimina tus archivos, filas de partidas, números de las clasificaciones, elección sobre estadísticas de la comunidad, perfil, sesiones, registro de seguridad y la propia cuenta. Las copias de seguridad y los registros de los proveedores caducan según el apartado 6.
- **Oponerte** al tratamiento basado en el interés legítimo, o pedirnos que **limitemos** el tratamiento.
- **Dejar de publicar** tu perfil y salir de las clasificaciones desactivando el interruptor.
- **Retirar tu consentimiento** para las estadísticas de héroes de la comunidad desactivando «Incluir mis partidas en las estadísticas de héroes de la comunidad»; tus partidas salen de los totales en la siguiente reconstrucción diaria.

Escribe a privacy@tavernledger.net para cualquier cosa que el sitio no haga por sí solo. Respondemos en el plazo de un mes.

También puedes presentar una reclamación ante una autoridad de protección de datos, en particular la del país de la UE donde vives o trabajas o donde crees que ocurrió el problema. La autoridad del responsable es la Comisión búlgara para la Protección de Datos Personales (Комисия за защита на личните данни), www.cpdp.bg.

## 8. Cookies y almacenamiento del navegador

El sitio web no instala cookies propias y no usa rastreo. Cuando inicias sesión, la biblioteca de Supabase almacena tu sesión de inicio de sesión en el almacenamiento local de tu navegador (clave `sb-<project>-auth-token`: tokens de acceso y de renovación y los datos básicos de tu cuenta) y, mientras te registras o restableces tu contraseña, un verificador de código de un solo uso (`…-code-verifier`) que demuestra que el enlace del correo volvió al navegador que lo solicitó. Son estrictamente necesarios: la sesión se elimina cuando cierras sesión, y un verificador de código cuando se usa su enlace o lo sustituye uno más reciente. Nuestro proveedor de alojamiento, Cloudflare, instala una cookie solo si su protección contra ataques te muestra una comprobación: `cf_clearance`, que registra que la superaste para que no se te vuelva a pedir durante un tiempo. Dura 30 minutos (ajuste de nuestra zona), es estrictamente necesaria y no se usa para rastrearte. La cookie de bots de Cloudflare `__cf_bm` no se usa en este sitio (sus funciones de bots están desactivadas), y el Network Error Logging de Cloudflare (informes del navegador sobre conexiones fallidas) está desactivado.

## 9. Menores

Debes tener 16 años o más para crear una cuenta. Si eres más joven, por favor no te registres. Si nos enteramos de que una cuenta pertenece a alguien menor de 16 años, la borramos.

## 10. Seguridad

Tu contraseña se almacena solo como hash. El sitio solo funciona mediante HTTPS. Cada usuario puede leer solo sus propios datos y los perfiles públicos; los datos de las partidas solo puede escribirlos nuestro servidor. El código es de código abierto, así que cualquiera puede comprobar cómo funciona.

## 11. Cambios

Publicaremos aquí los cambios con una fecha nueva. Si un cambio afecta a lo que recogemos o a por qué lo hacemos, avisaremos por correo electrónico a los usuarios registrados antes de que se aplique.

<p align="center"><a href="../README.md">English</a> · <a href="README.ru.md">Русский</a> · <a href="README.pt-BR.md">Português</a> · <b>Español</b> · <a href="README.fr.md">Français</a> · <a href="README.de.md">Deutsch</a></p>

<p align="center">
  <img src="../assets/icon.svg" width="112" alt="El logo de Decoy: un pato blanco con pico amarillo">
</p>

<h1 align="center">Decoy</h1>

<p align="center">
  <b>Muestra lo que quieras como tu estado de Discord.</b><br>
  Un juego al que no estás jugando, un botón que lleva a tu servidor, tu stream, tu propia imagen y tu propio texto.<br>
  Gratis y pequeño. Sin cuenta, sin contraseña, sin programar.
</p>

<p align="center">
  <a href="https://github.com/mountain-beaver/decoy/releases/latest/download/Decoy-windows-x64-setup.exe"><img src="download-windows.png" width="250" alt="Descargar para Windows"></a>
  &nbsp;
  <a href="https://github.com/mountain-beaver/decoy/releases/latest/download/Decoy-darwin-universal.dmg"><img src="download-macos.png" width="250" alt="Descargar para macOS"></a>
  &nbsp;
  <a href="https://github.com/mountain-beaver/decoy/releases/latest/download/Decoy-linux-amd64.AppImage"><img src="download-linux.png" width="250" alt="Descargar para Linux"></a>
</p>

<p align="center">
  <sub>Windows 10 y 11, macOS 10.15 o más reciente, la mayoría de los sistemas Linux. <a href="#install">Ayuda para instalar y otros archivos</a></sub>
</p>

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="window-dark.png">
    <img src="window-light.png" width="880" alt="La ventana de Decoy: rellenas el formulario a la izquierda y ves tu estado de Discord a la derecha">
  </picture>
</p>

Decoy es una app gratuita y de código abierto para el Rich Presence personalizado de Discord. Pone tu estado de actividad en Discord (la línea «Jugando a…» debajo de tu nombre) con cualquier juego, o con tu propio nombre, texto, imágenes y botones. Si conoces CustomRP para Windows, Decoy hace el mismo trabajo y además funciona en Mac y Linux.

## Lo que ven tus amigos

<p align="center">
  <img src="discord.png" width="720" alt="Tres fragmentos de una ventana real de Discord: la tarjeta de Jugando en un perfil, una fila de canal de voz con el icono del juego junto al nombre y una fila de la lista de miembros">
</p>

Son fragmentos de una ventana real de Discord, con el nombre y el avatar ocultos. Tu estado aparece en tu perfil, en la lista de miembros y como un icono pequeño junto a tu nombre en los canales de voz.

## Para qué lo usa la gente

<p align="center">
  <img src="ideas.png" width="860" alt="Tres estados de ejemplo: un juego de carreras, un servidor llamado Moonlight Lounge con un botón Únete a mi servidor y una radio en directo con dos botones">
</p>

- **Parecer ocupado.** Elige entre unos 25 000 juegos que Discord conoce. Tu estado se ve igual que si estuvieras jugando.
- **Invitar gente a tu servidor.** Pon en tu perfil un botón que abre tu enlace de invitación. [Así se hace](#add-a-link-to-your-server).
- **Compartir lo que te gusta.** Un stream, una lista de reproducción, un canal de YouTube, tu sitio web: cualquier enlace puede ser un botón.
- **Hacerlo a tu manera.** Tu propio nombre, imagen y texto en lugar de un juego. Una guía dentro de Decoy te lo explica paso a paso.

## Primeros pasos

1. **Descarga Decoy** con uno de los botones de arriba y ábrelo. La primera vez, Windows o macOS pueden preguntarte si confías en él: [qué pulsar](#install).
2. **Elige un juego.** Empieza a escribir su nombre y elígelo de la lista.
3. **Escribe tu texto**, si quieres: dos líneas, un temporizador, un botón. La vista previa de la derecha muestra lo que verán los demás.
4. **Pulsa «Show in Discord»** («Mostrar en Discord»). Es el botón amarillo de la franja encima del editor (o el interruptor de arriba a la derecha). Hasta que lo pulses, no aparece nada en tu perfil. Cuando funciona, el interruptor dice «Showing in Discord».

Discord tiene que estar abierto en el mismo ordenador (la app, no la web). Puedes cerrar la ventana de Decoy: sigue funcionando en la bandeja, junto al reloj.

<a id="add-a-link-to-your-server"></a>

## Añade un enlace a tu servidor

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="buttons-dark.png">
    <img src="buttons-light.png" width="680" alt="La sección Buttons de Decoy: la etiqueta dice Join my server y el enlace es una invitación de Discord">
  </picture>
</p>

1. En Discord, abre tu servidor, haz clic en su nombre arriba y elige **Invitar gente**. Haz clic en **Edit invite link** (editar enlace de invitación), configúralo para que nunca caduque y copia el enlace.
2. En Decoy, baja hasta **Buttons** (botones) y pulsa **Add button** (añadir botón).
3. En **Label** (etiqueta), escribe lo que debe decir el botón, por ejemplo `Únete a mi servidor`. En **Link** (enlace), pega tu invitación.
4. Comprueba que el interruptor de arriba a la derecha dice **Showing in Discord**. Si dice **Show in Discord**, púlsalo.

Es bueno saber que:

- No puedes hacer clic en tu propio botón. Discord solo deja que lo pulsen otras personas, así que pide a un amigo que lo pruebe.
- Puedes tener dos botones. Sirve cualquier enlace que empiece por `https://`: una invitación a un servidor, Twitch, YouTube, tu sitio web.
- ¿Quieres el nombre y el logo de tu servidor en lugar de un juego? Pulsa **Guide** (guía) en Decoy y luego **Own app** (app propia). Es gratis y lleva unos cinco minutos.

<a id="install"></a>

## Instalación

| Tu ordenador | Descarga | La primera vez que lo abres |
|---|---|---|
| Windows 10 y 11 | [Descargar .exe](https://github.com/mountain-beaver/decoy/releases/latest/download/Decoy-windows-x64-setup.exe) | Windows puede mostrar «Windows protegió su PC». Pulsa **Más información** y luego **Ejecutar de todas formas**. |
| Mac (Intel y Apple Silicon) | [Descargar .dmg](https://github.com/mountain-beaver/decoy/releases/latest/download/Decoy-darwin-universal.dmg) | Arrastra Decoy a Aplicaciones. Si macOS dice que no puede comprobar la app, abre **Ajustes del Sistema**, **Privacidad y seguridad**, baja hasta el final y pulsa **Abrir igualmente**. |
| Linux, cualquiera | [Descargar .AppImage](https://github.com/mountain-beaver/decoy/releases/latest/download/Decoy-linux-amd64.AppImage) | Permite que el archivo se ejecute (`chmod +x`) y luego ábrelo. |
| Debian, Ubuntu, Mint | [Descargar .deb](https://github.com/mountain-beaver/decoy/releases/latest/download/Decoy-linux-amd64.deb) | `sudo apt install ./Decoy-linux-amd64.deb` |
| Fedora, openSUSE | [Descargar .rpm](https://github.com/mountain-beaver/decoy/releases/latest/download/Decoy-linux-x86_64.rpm) | `sudo dnf install ./Decoy-linux-x86_64.rpm` |

**¿Por qué Windows y macOS muestran una advertencia?** Avisan de toda app cuyo creador no ha pagado un certificado a Microsoft o a Apple, y Decoy es un proyecto gratuito que no lo ha pagado. Los instaladores se generan automáticamente a partir del código de este repositorio, que cualquiera puede leer.

Todas las versiones están en la página de [Releases](https://github.com/mountain-beaver/decoy/releases).

## Preguntas

**¿Es gratis?**<br>
Sí. Sin anuncios y sin versión de pago.

**¿Es seguro para mi cuenta?**<br>
Decoy nunca te pide tu contraseña de Discord y no inicia sesión en tu cuenta. Solo le dice a la app de Discord de tu ordenador qué mostrar, igual que lo hacen los juegos.

**¿Tengo que dejarlo abierto?**<br>
Decoy tiene que estar en marcha, pero puedes cerrar su ventana: se queda en la bandeja. Para que arranque junto con tu ordenador, abre **Settings** (ajustes, el engranaje) y activa **Start with my computer** (iniciar con mi ordenador).

**¿Funciona en el móvil o en el navegador?**<br>
Decoy funciona en un ordenador, junto a la app de escritorio de Discord. Las personas que usan el móvil sí verán tu estado.

**No aparece nada en Discord. ¿Qué compruebo?**
1. En Decoy, el interruptor de arriba a la derecha dice **Showing in Discord**. Si dice **Show in Discord**, púlsalo.
2. La app de escritorio de Discord está en marcha en el mismo ordenador.
3. En Discord, en **Ajustes de usuario**, **Activity Privacy** (privacidad de la actividad): compartir tu actividad está activado.
4. Tu estado no está en **Invisible**.

Dentro de Decoy hay una lista de comprobación más larga: **Guide** y luego **Not showing** (no se muestra).

**¿Puedo tener más de un estado?**<br>
Sí. Crea varios perfiles y cambia entre ellos en la ventana o desde el icono de la bandeja.

**¿Puedo cambiar el nombre que aparece después de «Jugando a»?**<br>
Sí, con tu propia aplicación: **Guide** y luego **Own app**.

## Privacidad

Decoy no tiene cuenta, ni analíticas, ni un servidor propio. Tus perfiles se guardan solo en tu ordenador.

Se conecta a internet por dos motivos: a Discord, para cargar la lista de juegos, los nombres y las imágenes, y a GitHub, como máximo dos veces al día, para ver si hay una versión más nueva. Solo te muestra un enlace y nunca descarga nada por sí solo. Puedes desactivar esa comprobación en **Settings**.

## ¿Te gusta?

Dale una estrella al repositorio arriba en esta página. No cuesta nada y ayuda a que otras personas encuentren Decoy. ¿Encontraste un error o tienes una idea? [Cuéntamelo aquí](https://github.com/mountain-beaver/decoy/issues/new/choose), sin necesidad de detalles técnicos.

## Para desarrolladores

Decoy es de código abierto con [licencia MIT](../LICENSE). Cómo funciona por dentro, cómo compilarlo y cómo ejecutar las comprobaciones: [DEVELOPMENT.md](DEVELOPMENT.md).

<sub>Decoy no tiene ninguna relación con Discord ni con ninguna editora de juegos. Los nombres y los iconos de los juegos, incluidos los de las imágenes de arriba, pertenecen a sus dueños.</sub>

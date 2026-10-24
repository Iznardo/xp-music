# Preguntas frecuentes

## ¿Es oficial?

No. XP Music no está afiliado, patrocinado ni respaldado por Google, YouTube ni Microsoft. «YouTube Music», «Windows Media Player» y «Windows XP» son marcas de sus respectivos dueños. La app usa tu propia cuenta para lo que tú harías desde el navegador.

## Windows dice que la app puede ser peligrosa

El instalador está firmado, pero no con un certificado comercial, así que SmartScreen avisa las primeras veces. Pulsa «Más información» → «Ejecutar de todas formas».

## macOS dice que no puede abrir la app

La app de macOS es experimental y lleva una firma *ad hoc*, sin certificado de Apple ni notarización, así que Gatekeeper la bloquea la primera vez. En macOS 15 o posterior, intenta abrirla y luego ve a **Ajustes del Sistema → Privacidad y seguridad → «Abrir igualmente»**; en versiones anteriores, clic derecho → **Abrir**. Más detalles en [Instalar](install.md).

## En macOS no me llegan las actualizaciones

Es intencionado mientras macOS sea experimental: descarga el `.dmg` nuevo y sustituye la app en *Aplicaciones*. Tras cambiar de versión, macOS pedirá una vez permiso para que la app lea su sesión del Llavero.

## ¿Dónde se guarda mi sesión?

En Windows, cifrada con DPAPI (ligada a tu usuario). En Linux, en el Secret Service (GNOME Keyring, KeePassXC); si no está disponible, la sesión no se guarda en texto plano. En macOS, en el Llavero.

## En Linux no puedo iniciar sesión o se me olvida la cuenta

Comprueba que tienes un Secret Service desbloqueado (GNOME Keyring o KeePassXC con esa integración activada).

## En Linux cerrar la ventana cierra la app

Es intencionado: en muchos escritorios (como GNOME sin extensión AppIndicator) el icono de la bandeja no se ve. Si el tuyo lo muestra, activa «Cerrar a la bandeja» en Opciones.

## Una canción no suena

XP Music obtiene el audio con [yt-dlp](https://github.com/yt-dlp/yt-dlp). Si YouTube cambia algo, prueba **Opciones → Acerca de → Buscar actualización de yt-dlp**. Si sigue fallando, abre la carpeta de registros desde el mismo sitio y adjunta el archivo al abrir una *issue*.

## ¿Cómo cambio el idioma?

XP Music sigue el idioma de tu sistema (español si es español, inglés en cualquier otro caso). Para cambiarlo, ve a **Opciones → Reproductor → Idioma**.

## ¿Dónde informo de un error?

En las [issues de GitHub](https://github.com/Iznardo/xp-music/issues).

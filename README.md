# XP Music

Reproductor de escritorio para **YouTube Music** con el aspecto de Windows Media Player 9 en Windows XP. Para Windows 10/11 y Linux x86_64 (soporte experimental).

![Biblioteca de XP Music](docs/biblioteca.jpg)

![Reproduciendo con visualización](docs/reproduciendo.jpg)

- Inicio, búsqueda, biblioteca, «Me gusta», historial y radio de tu cuenta de YouTube Music.
- Listas de reproducción: crear, añadir canciones y reproducirlas.
- Visualizaciones que siguen el ritmo de la música.
- Modo compacto y **máscaras de Windows Media Player** (`.wmz`).
- Teclas multimedia, icono en la bandeja, controles en la miniatura de la barra de tareas y actualizaciones automáticas.

> XP Music no está afiliado, patrocinado ni respaldado por Google, YouTube ni Microsoft. «YouTube Music», «Windows Media Player» y «Windows XP» son marcas de sus respectivos dueños. La app usa tu propia cuenta para lo que tú harías desde el navegador.

**Web:** [iznardo.github.io/xp-music](https://iznardo.github.io/xp-music/)

## Instalar

Descarga la última versión desde [Releases](https://github.com/Iznardo/xp-music/releases/latest):
el instalador `.exe` para Windows, o el `.deb` / `.AppImage` para Linux. En NixOS hay un flake.
Los detalles (avisos de SmartScreen, requisitos en Linux, Nix, actualizaciones) están en [docs/instalar.md](docs/instalar.md).

## Documentación

- [Instalar](docs/instalar.md)
- [Uso](docs/uso.md): vistas, atajos, visualizaciones y opciones.
- [Máscaras de Windows Media Player](docs/mascaras.md)
- [Preguntas frecuentes](docs/preguntas-frecuentes.md)
- [Compilar](docs/compilar.md)
- [Publicar una versión](docs/publicar.md) (mantenedor)

## Terceros

- [yt-dlp](https://github.com/yt-dlp/yt-dlp) (Unlicense) y [Deno](https://deno.com) (MIT) van incluidos en el instalador para obtener el audio.
- [ytmapi-rs](https://github.com/nick42d/youtui) (MIT), con un parche, en `vendor/` (ver `vendor/README.md`).
- [Tauri](https://tauri.app), [Svelte](https://svelte.dev) y el resto de dependencias de `Cargo.toml` y `package.json`, cada una con su licencia.

## Licencia

[MIT](LICENSE) © 2026 iznardo.

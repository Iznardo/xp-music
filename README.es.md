<p align="right"><a href="README.md">English</a> · <b>Español</b></p>

<div align="center">
<img src="src-tauri/icons/128x128@2x.png" alt="Icono de XP Music" width="128">
<h1>XP Music</h1>
<p><b>Reproductor de escritorio para YouTube Music con el aspecto de Windows Media Player 9 en Windows XP.</b></p>
<p>
  <a href="https://github.com/Iznardo/xp-music/releases/latest"><img alt="Latest release" src="https://img.shields.io/github/v/release/Iznardo/xp-music?label=release"></a>
  <a href="https://github.com/Iznardo/xp-music/releases"><img alt="Downloads" src="https://img.shields.io/github/downloads/Iznardo/xp-music/total?label=downloads"></a>
  <a href="LICENSE"><img alt="License: MIT" src="https://img.shields.io/github/license/Iznardo/xp-music"></a>
  <img alt="Platforms: Windows, macOS, Linux" src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-0053ee">
  <a href="https://github.com/Iznardo/xp-music/actions/workflows/release.yml"><img alt="Build status" src="https://github.com/Iznardo/xp-music/actions/workflows/release.yml/badge.svg"></a>
  <a href="https://iznardo.github.io/xp-music/"><img alt="Web" src="https://img.shields.io/badge/Web-iznardo.github.io%2Fxp--music-3a9d23"></a>
</p>
</div>

Para Windows 10/11 (x64 y ARM64), con soporte experimental para Linux (x64 y ARM64) y macOS (Intel y Apple Silicon).

![Biblioteca de XP Music](docs/library.es.jpg)

![Reproduciendo con visualización](docs/now-playing.es.jpg)

- Inicio, búsqueda, biblioteca, «Me gusta», historial y radio de tu cuenta de YouTube Music.
- Listas de reproducción: crear, añadir canciones y reproducirlas.
- Visualizaciones que siguen el ritmo de la música.
- Modo compacto y **máscaras de Windows Media Player** (`.wmz`).
- Teclas multimedia, icono en la bandeja, controles en la miniatura de la barra de tareas y actualizaciones automáticas.
- Interfaz en español e inglés, según el idioma de tu sistema.

> XP Music no está afiliado, patrocinado ni respaldado por Google, YouTube ni Microsoft. «YouTube Music», «Windows Media Player» y «Windows XP» son marcas de sus respectivos dueños. La app usa tu propia cuenta para lo que tú harías desde el navegador.

**Web:** [iznardo.github.io/xp-music](https://iznardo.github.io/xp-music/)

## Instalar

Descarga la última versión desde [Releases](https://github.com/Iznardo/xp-music/releases/latest):
el instalador `.exe` para Windows, el `.deb` / `.rpm` / `.AppImage` para Linux o el `.dmg` para macOS. En Arch hay un `PKGBUILD` y en NixOS un flake.
Los detalles (avisos de SmartScreen y Gatekeeper, requisitos en Linux, Arch, Nix, actualizaciones) están en [docs/es/install.md](docs/es/install.md).

## Documentación

- [Instalar](docs/es/install.md)
- [Uso](docs/es/usage.md): vistas, atajos, visualizaciones y opciones.
- [Máscaras de Windows Media Player](docs/es/skins.md)
- [Visualizaciones propias](docs/es/visualizations.md)
- [Preguntas frecuentes](docs/es/faq.md)
- [Compilar](docs/es/building.md)
- [Publicar una versión](docs/es/releasing.md) (mantenedor)

## Terceros

- [yt-dlp](https://github.com/yt-dlp/yt-dlp) (Unlicense) y [Deno](https://deno.com) (MIT) van incluidos en los instaladores y paquetes de las releases para obtener el audio.
- [ytmapi-rs](https://github.com/nick42d/youtui) (MIT), con un parche, en `vendor/` (ver `vendor/README.md`).
- [Tauri](https://tauri.app), [Svelte](https://svelte.dev) y el resto de dependencias de `Cargo.toml` y `package.json`, cada una con su licencia.

## Licencia

[MIT](LICENSE) © 2026 iznardo.

# XP Music

Reproductor de escritorio para **YouTube Music** con el aspecto de Windows Media Player 9 en Windows XP. Solo para Windows 10 y 11.

![Biblioteca de XP Music](docs/biblioteca.jpg)

![Reproduciendo con visualización](docs/reproduciendo.jpg)

- Inicio, búsqueda, biblioteca, «Me gusta», historial y radio de tu cuenta de YouTube Music.
- Listas de reproducción: crear, añadir canciones y reproducirlas.
- Visualizaciones que siguen el ritmo de la música.
- Modo compacto y **máscaras de Windows Media Player** (`.wmz`).
- Teclas multimedia, icono en la bandeja, controles en la miniatura de la barra de tareas y actualizaciones automáticas.

> XP Music no está afiliado, patrocinado ni respaldado por Google, YouTube ni Microsoft. «YouTube Music», «Windows Media Player» y «Windows XP» son marcas de sus respectivos dueños. La app usa tu propia cuenta para lo que tú harías desde el navegador.

## Instalar

1. Descarga el instalador (`XP Music_x.y.z_x64-setup.exe`) de la página de *releases*.
2. Ábrelo. Como no lleva una firma comercial, Windows SmartScreen puede avisar la primera vez: «Más información» → «Ejecutar de todas formas».
3. Inicia sesión con tu cuenta de Google desde la propia app.

La app se actualiza sola: cuando hay una versión nueva, te lo ofrece al arrancar.

## Máscaras de Windows Media Player

XP Music lee las máscaras originales de WMP (`.wmz`). No incluye ninguna: copia las que tengas en la carpeta de máscaras (menú **Ver → Selector de máscaras… → Abrir carpeta**, que es `%APPDATA%\xp-music\skins`) y pulsa **Actualizar**.

Para volver al aspecto normal: **Ctrl+1**, clic derecho sobre la máscara → «Volver al modo completo», o desde el icono de la bandeja.

Los scripts de cada máscara se ejecutan aislados, sin acceso a tu cuenta ni a tus archivos. Las ventanas secundarias (ecualizador, lista aparte) todavía no se abren.

## Compilar

Requisitos: Windows, [Rust](https://rustup.rs) estable, Node.js 20 o posterior y WebView2 (viene con Windows 11).

```bash
npm install
npm run fetch-tools    # descarga yt-dlp y deno en tools/ (versiones fijadas en tools.lock.json)
npm run tauri dev      # la app en modo desarrollo
```

Otras órdenes útiles:

- `npm run dev:mock`: la interfaz en el navegador con datos simulados, sin backend.
- `npm run check`: comprobaciones de TypeScript, formato, Clippy y tests de Rust.
- `npm run release`: compila, firma y publica una versión (ver `scripts/release.ps1`).

## Terceros

- [yt-dlp](https://github.com/yt-dlp/yt-dlp) (Unlicense) y [Deno](https://deno.com) (MIT) van incluidos en el instalador para obtener el audio.
- [ytmapi-rs](https://github.com/nick42d/youtui) (MIT), con un parche, en `vendor/` (ver `vendor/README.md`).
- [Tauri](https://tauri.app), [Svelte](https://svelte.dev) y el resto de dependencias de `Cargo.toml` y `package.json`, cada una con su licencia.

## Licencia

[MIT](LICENSE) © 2026 iznardo.

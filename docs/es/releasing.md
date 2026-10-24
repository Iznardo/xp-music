# Publicar una versión

Solo para el mantenedor.

## Quién compila qué

| Target | Quién lo compila | Firmas |
| --- | --- | --- |
| Windows x64 y ARM64 | **El equipo Windows del mantenedor** (`npm run release`; ARM64 con compilación cruzada) | Authenticode con el certificado local + updater |
| Linux x64 / ARM64 | CI (`ubuntu-24.04`, `ubuntu-24.04-arm`): `.deb`, AppImage, `.rpm` | Updater |
| macOS Intel / Apple Silicon | CI (`macos-15-intel`, `macos-15`): `.dmg` | Sin updater; *ad hoc* o Developer ID + notarización si hay secretos `APPLE_*` |

Se conserva el certificado Authenticode de Windows: `setup-signing.ps1` lo crea
**no exportable**, así que los instaladores de Windows (x64 y ARM64) se firman en el equipo
Windows del mantenedor y el CI los reutiliza byte a byte desde el draft (en el runner
`windows-11-arm` solo se instala y se prueba el de ARM64). Así la clave del updater tampoco
sale de ese equipo para Windows.

Para compilar ARM64 en un equipo x64 hacen falta, una vez, `rustup target add
aarch64-pc-windows-msvc` y el componente «MSVC ARM64 build tools» de Visual Studio Build
Tools. `npm run release` descarga el yt-dlp y el deno de ARM64 (`tools.lock.json`) y los
pone en `tools/` solo mientras compila ese instalador.

## Configuración (una vez)

En **Iznardo/xp-music → Settings**:

1. **Environments → New environment → `release`**, con **Required reviewers** (tú) y
   **Deployment branches and tags** limitado a la rama `main` (el workflow de release se
   lanza desde `main`). Los secretos van **solo** aquí, no en los secretos del
   repositorio: así ningún job los recibe sin tu aprobación.
2. **Secrets and variables → Actions → Variables** (del repositorio) para las dos variables.

| Dónde | Tipo | Nombre | Valor |
| --- | --- | --- | --- |
| Repositorio | Variable | `XP_MUSIC_UPDATER_PUBKEY` | Contenido de la clave pública existente `~/.xp-music/updater.key.pub` |
| Repositorio | Variable | `WINDOWS_SIGNING_THUMBPRINT` | Huella de `src-tauri/signing.local.json` |
| Environment `release` | Secret | `TAURI_SIGNING_PRIVATE_KEY` | Contenido de la clave privada **existente** `~/.xp-music/updater.key` (no una ruta) |
| Environment `release` | Secret | `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Su contraseña; vacío si no tiene |
| Environment `release` | Secret (opcional) | `APPLE_CERTIFICATE` | Certificado «Developer ID Application» exportado como `.p12`, en base64 |
| Environment `release` | Secret (opcional) | `APPLE_CERTIFICATE_PASSWORD` | Contraseña del `.p12` |
| Environment `release` | Secret (opcional) | `APPLE_SIGNING_IDENTITY` | Nombre de la identidad, p. ej. `Developer ID Application: Nombre (TEAMID)` |
| Environment `release` | Secret (opcional) | `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID` | Para notarizar: Apple ID, contraseña específica de app y Team ID |

No generes otra clave del updater: las versiones ya instaladas confían en la actual.
Los secretos solo se pasan al compilar releases, nunca en los checks de PR, ni en push
ni en el dry run, y solo a los jobs de Linux (clave del updater) y macOS (certificados
de Apple); cada release se detiene hasta que apruebes el environment `release` en
Actions → Review deployments. Windows no necesita ninguno: llega firmado desde tu equipo.
Sin `APPLE_CERTIFICATE` y `APPLE_SIGNING_IDENTITY`, macOS se firma *ad hoc* (los usuarios
deben saltarse Gatekeeper, ver [Instalar](install.md)); con ellos se firma con Developer ID,
y se notariza si además están `APPLE_ID`, `APPLE_PASSWORD` y `APPLE_TEAM_ID`.

## Lanzar la release

Desde `main` en el equipo Windows del mantenedor, con `origin` apuntando al upstream:

```powershell
npm run release                     # incrementa patch
npm run release -- -Bump minor
npm run release -- -Bump none -NoPublish  # prueba local, no publica ni lanza CI
npm run release -- -Notes "Qué cambia en esta versión"
```

El script compila y verifica los instaladores Windows x64 y ARM64, sube el tag y prepara una
**release draft** con los EXE, sus firmas del updater y las notas (`-Notes`). Después lanza
el workflow único:

1. Valida el tag, la versión, que sea un draft y que el commit esté en `main`, antes de
   usar ninguna clave. Solo se publica desde `Iznardo/xp-music`.
2. Compila, comprueba y prueba el arranque de los seis targets. En Windows no recompila:
   verifica la firma Authenticode de cada EXE del draft contra la huella configurada y
   lo prueba tal cual. Linux se compila con la misma clave pública y genera sus firmas
   del updater; macOS se compila sin updater.
3. Verifica criptográficamente con `minisign` **todas** las firmas del updater, sube los
   archivos al draft (salvo los EXE de Windows, que ya estaban) y calcula el SHA-256 de cada uno.
4. Escribe en las notas de la release, **encima** de las notas del mantenedor, una tabla
   *Sistema (SO y arquitectura) · SHA-256 · Descarga* con los instaladores; el texto del
   enlace es el nombre del archivo, que ya indica el formato. Los archivos del actualizador
   (`.sig`, `latest.json`) siguen en la release pero no se listan. La tabla va
   entre marcadores HTML, así que reintentar el workflow la sustituye en vez de duplicarla.
5. Crea un único `latest.json` (con solo las notas del mantenedor) con una entrada por
   target y formato: `windows-x86_64`, `windows-aarch64`, `linux-{x86_64,aarch64}-{deb,appimage,rpm}`
   (macOS no tiene entrada: es experimental y no se autoactualiza); las de Windows conservan
   también la clave sin formato (`windows-x86_64`…) que usan las instalaciones existentes.
6. Publica el draft solo cuando todo ha pasado.

Si falla, el draft no se publica. Corrige el problema y reintenta **el mismo tag**
desde Actions → Desktop → Run workflow, o:

```bash
gh workflow run release.yml --repo Iznardo/xp-music --ref main -f tag=v1.0.3
```

**Dry run:** lanzar el workflow *sin* tag (Actions → Run workflow con el campo vacío, o
`gh workflow run release.yml --ref mi-rama`) compila y prueba los seis targets sin
claves ni publicación; sirve para probar cambios de CI en cualquier rama.

No publiques el draft manualmente ni ejecutes otro proceso que escriba
`latest.json`: el workflow es el único publicador. Rechaza tags ajenos a `main`,
versiones inconsistentes, releases ya publicadas y la publicación desde forks.
Para activar el updater en una compilación Linux anterior sin clave pública,
instala primero uno de los nuevos paquetes firmados.

## Arch Linux

El `PKGBUILD` de `packaging/arch/` no forma parte de la publicación. Tras cada release,
sube `pkgver` (con `pkgrel=1`), ejecuta `updpkgsums` en esa carpeta y confirma el cambio.

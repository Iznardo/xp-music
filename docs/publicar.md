# Publicar una versión

Solo para el mantenedor.

Se conserva el certificado Authenticode de Windows: `setup-signing.ps1` lo crea
**no exportable**, así que el instalador sigue firmándose en el equipo Windows del
mantenedor. No se sustituye por un EXE sin firmar generado en CI.

Configura una vez en **Iznardo/xp-music → Settings → Secrets and variables → Actions**:

| Tipo | Nombre | Valor |
| --- | --- | --- |
| Variable | `XP_MUSIC_UPDATER_PUBKEY` | Contenido de la clave pública existente `~/.xp-music/updater.key.pub` |
| Variable | `WINDOWS_SIGNING_THUMBPRINT` | Huella de `src-tauri/signing.local.json` |
| Secret | `TAURI_SIGNING_PRIVATE_KEY` | Contenido de la clave privada **existente** `~/.xp-music/updater.key` (no una ruta) |
| Secret | `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Su contraseña; vacío si no tiene |

No generes otra clave del updater: las versiones ya instaladas confían en la actual.
Los secretos solo se usan al compilar releases, nunca en los checks de PR.

Desde `main` en el equipo Windows del mantenedor, con `origin` apuntando al upstream:

```powershell
npm run release                     # incrementa patch
npm run release -- -Bump minor
npm run release -- -Bump none -NoPublish  # prueba local, no publica ni lanza CI
```

El script compila y verifica el instalador Windows, sube el tag y prepara una
**release draft** con su EXE y firma del updater. Después lanza el workflow único:

1. Comprueba el código en Windows y Linux. Verifica la firma Authenticode del EXE
   original contra la huella configurada, sin recompilarlo ni cambiarlo.
2. Compila `.deb` y AppImage con la misma clave pública y genera sus firmas.
   Prueba ambos paquetes fuera del repositorio.
3. Verifica criptográficamente las tres firmas del updater, adjunta Linux y crea
   un único `latest.json` con Windows y entradas Linux separadas por formato.
4. Publica el draft solo cuando todo ha pasado. La entrada `windows-x86_64` se
   conserva para los clientes Windows existentes.

Si falla, el draft no se publica. Corrige el problema y reintenta **el mismo tag**
desde Actions → Desktop → Run workflow, o:

```bash
gh workflow run release.yml --repo Iznardo/xp-music --ref main -f tag=v1.0.3
```

No publiques el draft manualmente ni ejecutes otro proceso que escriba
`latest.json`: el workflow es el único publicador. Rechaza tags ajenos a `main`,
versiones inconsistentes, releases ya publicadas y la publicación desde forks.
Para activar el updater en una compilación Linux anterior sin clave pública,
instala primero uno de los nuevos paquetes firmados.


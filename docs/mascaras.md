# Máscaras de Windows Media Player

XP Music lee las máscaras originales de WMP (`.wmz`). No incluye ninguna: copia las que tengas en la carpeta de máscaras y pulsa **Actualizar**.

## Dónde van

Abre **Ver → Selector de máscaras… → Abrir carpeta**. Es:

| Sistema | Carpeta |
| --- | --- |
| Windows | `%APPDATA%\xp-music\skins` |
| Linux | `~/.config/xp-music/skins` (o `$XDG_CONFIG_HOME/xp-music/skins`) |

Se pueden organizar en subcarpetas.

## Volver al aspecto normal

- **Ctrl+1**.
- Clic derecho sobre la máscara → «Volver al modo completo».
- Desde el icono de la bandeja.

## Limitaciones

- Los scripts de cada máscara se ejecutan aislados, sin acceso a tu cuenta ni a tus archivos.
- Las ventanas secundarias (ecualizador, lista aparte) todavía no se abren.
- En Linux no funciona el clic a través de las zonas transparentes.

# Visualizaciones propias

Añade tus propias visualizaciones como archivos de shader GLSL. Aparecen en los botones ◀ ▶ de Reproducción actual y en **Herramientas → Opciones → Visualización**.

## Dónde van

Opciones → Visualización → **Abrir carpeta**. Es la carpeta `visualizations`, junto a `skins`:

| Sistema | Carpeta |
| --- | --- |
| Windows | `%APPDATA%\xp-music\visualizations` |
| Linux | `~/.config/xp-music/visualizations` (o `$XDG_CONFIG_HOME/xp-music/visualizations`) |
| macOS | `~/Library/Application Support/xp-music/visualizations` |

Cada archivo `*.frag` (sin subcarpetas, hasta 64 KB) es una visualización, con el nombre del archivo. Pulsa **Actualizar** tras añadir una.

## Cómo escribir una

El archivo es solo `void main()`. La cabecera de abajo se añade sola (WebGL2, GLSL ES 3.00), así que no escribas `#version` ni la redeclares:

| Nombre | Tipo | Qué es |
| --- | --- | --- |
| `uv` | `vec2` | Posición del píxel, 0..1 |
| `res` | `vec2` | Tamaño en píxeles |
| `bands[64]` | `float` | Espectro 0..1, de graves a agudos |
| `level` | `float` | Volumen general 0..1 |
| `beat` | `float` | Pulso que salta a 1 en cada golpe de bajo y decae |
| `bass` | `float` | Graves, normalizados a la canción |
| `time` | `float` | Segundos |
| `step` | `float` | Fotogramas a 60 fps desde el último dibujo. Úsalo para la velocidad de la estela: `pow(0.95, step)` |
| `prev` | `sampler2D` | El fotograma anterior, para estelas |
| `o` | `out vec4` | El color que escribes |

Ayudas: `hsv(h, s, v)`, `band(x)` (espectro en x de 0..1) y `avg(a, b)` (media de las bandas `a` a `b-1`).

```glsl
// pulse.frag: un anillo que respira con el bajo y deja estela.
void main() {
  vec2 c = uv - 0.5;
  c.x *= res.x / res.y;
  vec3 trail = texture(prev, uv).rgb * pow(0.9, step);
  float ring = smoothstep(0.02, 0.0, abs(length(c) - 0.15 - bass * 0.2));
  o = vec4(max(trail, hsv(time * 0.05, 0.8, 1.0) * ring), 1.0);
}
```

## Si no funciona

- Un shader que no compila se sustituye por las barras, y el error va al registro (Opciones → Acerca de → registros).
- Los shaders corren en tu GPU sin límite: uno muy pesado irá lento. Instala solo los que te fíes.
- Las visualizaciones nativas de WMP (`.dll`) no están soportadas.

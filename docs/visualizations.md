# Custom visualizations

Add your own visualizations as GLSL shader files. They show up in the ◀ ▶ buttons of Now Playing and in **Tools → Options → Visualization**.

## Where they go

Options → Visualization → **Open folder**. It is the `visualizations` folder next to `skins`:

| System | Folder |
| --- | --- |
| Windows | `%APPDATA%\xp-music\visualizations` |
| Linux | `~/.config/xp-music/visualizations` (or `$XDG_CONFIG_HOME/xp-music/visualizations`) |
| macOS | `~/Library/Application Support/xp-music/visualizations` |

Every `*.frag` file (no subfolders, up to 64 KB each) is one visualization, named after the file. Press **Refresh** after adding one.

## Writing one

The file is only `void main()`. The header below is added for you (WebGL2, GLSL ES 3.00), so do not write `#version` or redeclare it:

| Name | Type | What it is |
| --- | --- | --- |
| `uv` | `vec2` | Pixel position, 0..1 |
| `res` | `vec2` | Size in pixels |
| `bands[64]` | `float` | Spectrum 0..1, bass to treble |
| `level` | `float` | Overall volume 0..1 |
| `beat` | `float` | Pulse that jumps to 1 on each bass hit and decays |
| `bass` | `float` | Bass, normalized to the song |
| `time` | `float` | Seconds |
| `step` | `float` | Frames at 60 fps since the last draw. Use it for trail speed: `pow(0.95, step)` |
| `prev` | `sampler2D` | The previous frame, for trails |
| `o` | `out vec4` | The color you write |

Helpers: `hsv(h, s, v)`, `band(x)` (spectrum at x in 0..1) and `avg(a, b)` (mean of bands `a` to `b-1`).

```glsl
// pulse.frag: a ring that breathes with the bass and leaves a trail.
void main() {
  vec2 c = uv - 0.5;
  c.x *= res.x / res.y;
  vec3 trail = texture(prev, uv).rgb * pow(0.9, step);
  float ring = smoothstep(0.02, 0.0, abs(length(c) - 0.15 - bass * 0.2));
  o = vec4(max(trail, hsv(time * 0.05, 0.8, 1.0) * ring), 1.0);
}
```

## If it does not work

- A shader that does not compile is replaced by the bars, and the error goes to the log (Options → About → logs).
- Shaders run on your GPU with nothing limiting them: a very heavy one will be slow. Only install shaders you trust.
- Native WMP visualizations (`.dll`) are not supported.

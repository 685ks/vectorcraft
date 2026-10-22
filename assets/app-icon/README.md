# VectorCraft app icon

A Bézier curve with its anchors and handles, drawn as a simple line glyph in the Arthavan Projects house style.
Every app shares the same tile shape and glyph style, so the family reads as one set; each app has
its own muted tint (hue 335°), and the glyph tells them apart.

## Palette

The house style's dark tile and light accent, tinted to this app's hue:

| Colour | Hex | Used for |
|---|---|---|
| Tile | `#4d2335` → `#1a0a11` | Vertical gradient filling the rounded square |
| Hairline | white at 14% | Inner edge of the tile |
| Accent | `#e198b7` | The glyph's strokes |
| Accent bright | `#f4c2d7` | Highlights in the glyph |
| Accent low | `#7f3956` | Soft fills behind the glyph |

## Geometry

A 512 × 512 viewBox, clipped to a rounded square with `rx=112`; glyph strokes are 30 units with
round caps, so they stay legible at 16-32 px. Windows and Linux use the full tile. macOS files
sit on Apple's grid (an 824/1024 body with a transparent margin, made by widening the viewBox).

## Provenance and licence

Original work: written by hand as SVG for this project, no source image traced or copied.
MIT OR Apache-2.0, like the code. `vectorcraft.svg` is the master; `packaging/icons.sh` renders every
other file here from it.

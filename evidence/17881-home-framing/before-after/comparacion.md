# #17881 measured before and after #17921

Both runs are the real desktop app, on the same machine, over the app's own
control channel: `ui.worldToScreen` on the middle of the part (0, 0, 20) after
the camera settled, and `ui.inspect` for the viewport rectangle.

The part is the one from the issue: a 17 × 17 × 40 body centred on the Z axis.
The window is 1600 px wide and the Browser is docked on the left; its card ends
at x = 249, so the area the panels leave free is x = 250..1600, whose middle
is x = 925.

| build | Browser docked left, after Home | Browser hidden, after Home |
|---|---|---|
| `3f886e460^` (before #17921) | part's middle at x = 1177.8 | part's middle at x = 1177.8 |
| `3f886e460` (after #17921)  | part's middle at x = 925.0 | part's middle at x = 800.0 |

Before the fix, Home leaves the part at 1177.8, which is 74% of the window
width and 252 px to the right of the middle of the free area: the shift the
issue reports. It does not move when the Browser is hidden, because the
panels were not part of the framing area at all.

After the fix the part lands at 925.0, the middle of the area the panels leave
free, and at 800.0, the middle of the whole view, once the Browser is hidden.

The "before" build is `3f886e460^` (`04c3a24e`) and the "after" build is
`3f886e460`, both built from source on this machine.

Files:

- `before-01-navegador-abierto-antes-de-home.png`: before, Browser docked left,
  before Home.
- `before-02-navegador-abierto-despues-de-home.png`: before, Browser docked
  left, after Home. The part is off to the right.
- `before-03-sin-navegador-despues-de-home.png`: before, Browser hidden, after
  Home. The part is in the same place: the panels were never in the framing
  area.
- `after-01-navegador-abierto-antes-de-home.png`: after, Browser docked left,
  before Home.
- `after-02-navegador-abierto-despues-de-home.png`: after, Browser docked left,
  after Home. The part is in the middle of the area the panels leave free.
- `after-03-sin-navegador-despues-de-home.png`: after, Browser hidden, after
  Home. The part is in the middle of the whole view.
- `before-estado.json`, `after-estado.json`: the raw measurements.

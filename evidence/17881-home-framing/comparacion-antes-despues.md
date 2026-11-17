# Before / after of the real app, measured on this machine

The three PNGs of this directory were taken with the desktop build of this
branch (the head `cb83606210f6c341b6cecc81e9ed9ad77a46e9c0`, with the fix).

The same three captures were taken with the same worktree rebuilt WITHOUT the
fix (the parent of the fix commit, `166b487a3^`, the two production files
reverted and the app binary rebuilt). The numbers came out of the app's own
control channel (`ui.worldToScreen` on the middle of the part, `ui.inspect`
for the viewport).

| build | Browser docked left, after Home | Browser hidden, after Home |
|-------|----------------------------------|-----------------------------|
| with the fix    | part's middle at x = 925.0 | part's middle at x = 800.0 |
| without the fix | part's middle at x = 925.0 | part's middle at x = 800.0 |

The three PNGs of the two runs are byte for byte the same (md5 equal).

Why: on this machine the desktop app runs the path WITH the shared panel
host, and there the framing area is computed by `shell::host_usable`, which
already leaves the panels out (`occluding_rects`). The change of this pull
request is in the path WITHOUT a shared host, where `Layout::with_content`
was called with no obstacles: there the fix aligns the app's own layout with
the host's. That path is the one the regression test
(`home_frames_the_part_in_the_area_the_panels_leave_free`) covers.

What the captures of this directory show, then, is the behaviour the issue
asks for in the real app: with the Browser docked on the left (its card ends
at x = 249, so the area the panels leave free is x = 250..1600, whose middle
is x = 925) Home puts the middle of the part at x = 925.0, and with the
Browser hidden at x = 800.0, the middle of the whole view.

Files:

- `17881-01-navegador-abierto-antes-de-home.png`: the Browser docked on the
  left, before Home.
- `17881-02-navegador-abierto-despues-de-home.png`: the Browser docked on the
  left, after Home. The part is in the middle of the area the panels leave
  free (x = 925 of 250..1600), not in the middle of the window.
- `17881-03-sin-navegador-despues-de-home.png`: the same part after Home with
  the Browser hidden: the part is in the middle of the whole view (x = 800).
- `17881-estado.json`: what the control channel answered for each of them.

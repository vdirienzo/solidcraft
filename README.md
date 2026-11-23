# Evidence for #16994

The view with the line being drawn, before and after the fix. The ring is where the pointer
is, the band is where the line's end goes.

- `sketch-line-end-before.png` / `sketch-line-end-before-zoom.png`: the band's end leaves the
  pointer and lands on the horizontal through the start, 1.57 mm away.
- `sketch-line-end-after.png` / `sketch-line-end-after-zoom.png`: the band's end stays at the
  pointer.

The pictures come from the test `evidence_the_line_end_lands_away_from_the_pointer` of the
pull request (run it with `SOLIDCRAFT_16994_EVIDENCE=<folder>`).

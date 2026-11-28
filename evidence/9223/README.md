# Issue #9223 - evidence

Captured from the app's own window (1600x1000) under Xvfb, driven through the control channel
(`ui.editFeature`, `engine.execute`, `ui.click`, `ui.screenshot`).

## A parameter added while the feature editor is open

- `parameter-01-editor-with-the-parameter-before.png` - the extrude's editor is open and the
  parameter `len` (20 mm) has just been added while it was open.
- `parameter-02-after-ok-before.png` - after OK, on the base commit: the editor refuses with
  "the design changed while the feature editor was open" and the edit is lost.
- `parameter-03-editor-with-the-parameter-after.png` - the same editor, same parameter, on the
  branch.
- `parameter-04-after-ok-after.png` - after OK on the branch: the editor closes, the edit is
  applied (the extrude's distance is the parameter) and the design keeps the parameter.

## The reporter's own steps, on the current main

- `reporter-01-editor-open.png` - the extrude's editor is open (three of the ten circles extruded).
- `reporter-02-after-pattern-click.png` - the rectangular sketch pattern was clicked in the browser.
- `reporter-03-after-picks.png` - the seven other circles were picked into the Profiles field.
- `reporter-04-after-ok.png` - after OK: the edit is applied, all ten profiles are extruded and the
  design keeps ten bodies. This defect was fixed upstream in #10822 (#10687), released in 0.30.1.

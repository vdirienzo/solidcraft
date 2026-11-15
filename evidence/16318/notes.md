# The Agent chat's "Continue" button, for #16318

Pictures of the whole SolidCraft window, uncropped, taken from a build of the pull
request's head (`496aa27c3cc19e39642206e256230160d11486d9`).

| Picture | Size | What it shows |
|---|---|---|
| `01-cut-short.png` | 1280x720 | The turn was cut short by the provider's usage limit. The panel shows the error and, under it, "The agent stopped before finishing the task." with the **Continue** button. |
| `02-after-continue.png` | 1280x720 | Right after Continue was pressed. The turn is under way again ("Thinking for 0 s"), the Continue button is gone, and the task is **not** repeated in the conversation: the same message, continued. |
| `03-result.png` | 1280x720 | The continued turn finished: the agent built the part (Box 60 x 40 x 20 mm, Body1) and answered. There is nothing left to continue, so the Continue button is gone. |
| `04-small-window.png` | 960x600 | The same cut-short turn in the smallest window the app allows (its minimum inner size is 960x600). The notice and the button fit the panel: nothing overflows, nothing overlaps. |

## How they were taken

A local OpenAI-compatible endpoint stood in for the provider, so the cut-short turn is a
real one and the panel is not mocked:

- the endpoint listed one model (`evidence-model`) and refused the first chat-completions
  request with its usage-limit answer (HTTP 429, "You exceeded your current quota, please
  check your plan and billing details."), which is what the chat turns into a turn that
  ended with an error, with the task kept;
- the continued turn was answered normally. It asked for SolidCraft's own `execute` tool
  with `solid.box`, so the part in `03-result.png` was built by the app itself, and then
  closed the turn with its answer.

The app was driven over its control channel (`docs/control-protocol.md`), the same channel
the MCP server uses. The pictures are grabs of the X display (`x11grab` on the root of the
display the app ran on), so they are the window as a user sees it, with nothing trimmed.

## What the provider received

The endpoint's own log of the two turns, with the system note left out:

```text
turn 1  user: make a gearbox housing
        -> HTTP 429 (the provider's usage limit)

turn 2  user: [The previous attempt was cut short before it finished. Continue the task from where it stopped.]
                  make a gearbox housing
        -> the app's own execute tool: solid.box {length: 60, width: 40, height: 20}
        -> the agent's answer
```

The task goes again with the note in front of it, and the conversation shows it once.

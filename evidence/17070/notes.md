# The question card in the Assistant panel, for #17070

Pictures of the whole SolidCraft window, uncropped, taken from a build of the pull
request's head (`d11a9519cb2b314160fdb9f73b192af67d492dd6`).

| Picture | Size | What it shows |
|---|---|---|
| `01-question-with-choices.png` | 1280x720 | A question from a connected agent, in the Assistant panel: "The agent asks: Which approach should I take for the bracket?" with a button per choice (**conservative**, **balanced**, **aggressive**) and, next to them, **Decline** and **Cancel**. |
| `02-link-question.png` | 1280x720 | A link question from the same agent: "The agent asks: Open this page to let the agent connect to your account." with the address in full (`Address: https://agent.example.com/oauth/authorize?client_id=solidcraft&scope=design%20read%20write&state=7f3c1d`) and its **Open** button, plus **Decline** and **Cancel**. |
| `03-question-with-choices-small-window.png` | 960x600 | The same question card in the smallest window the app allows: it fits, nothing overflows and nothing overlaps. |
| `04-link-question-small-window.png` | 960x600 | The same link question in the smallest window the app allows: the address and the Open button fit, nothing overflows and nothing overlaps. |

The button row is laid out from the right, as the panel's other action rows are:
the first choice the agent offered is the rightmost button.

## How they were taken

The agent is a real ACP process: it speaks the Agent Client Protocol over
stdin/stdout, so the card is produced by the protocol path the change adds
(`elicitation/create`), not by a mock of the panel.

- the app was started with `--control PORT` and driven over its control channel
  (`docs/control-protocol.md`), the same channel the MCP server uses;
- the agent was added to the panel's agents (`ui.agent_chat {add_agent: …}`,
  `kind: "acp"`), picked and connected; it answered `initialize` and
  `session/new`, and the panel reported the state `ready`;
- a message was sent (`ui.agent_chat {send: …}`); the agent answered it with its
  own question, which the panel asked in the Assistant panel;
- the picture is the app's own `ui.screenshot` of the whole window, resized to
  1280x720 with `ui.resize` (and to 960x600 for the small-window check).

`measurement.json` has the size, the sha256 and the exact text of every picture.

## What the agent asked

```text
form  elicitation/create  mode=form  message="Which approach should I take for the bracket?"
                          requestedSchema.properties.approach.enum = ["conservative", "balanced", "aggressive"]

link  elicitation/create  mode=url   message="Open this page to let the agent connect to your account."
                          url="https://agent.example.com/oauth/authorize?client_id=solidcraft&scope=design%20read%20write&state=7f3c1d"
```

The exchange itself, as the agent read and wrote it, is in
`agent-protocol-form.log` and `agent-protocol-link.log`.

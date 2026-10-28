# Notes for coding agents

Read [doc/README.md](doc/README.md) and the page matching the task before you start: conventions, architecture and research results are there. Keep what you learn about this project in `doc/`, not in an agent's memory.

Commit every meaningful change as soon as it is done and the checks pass — one concern per commit, no stacking work into one big commit at the end.

## Writing in `doc/`

One line per entry: what the part is for and what to look for in it, as in `doc/architecture.md`. Why a choice was made — an iced quirk it works around, a colour rule — belongs in the code's doc comments and the commit message.

## Checking a UI change on screen

`agent-screenshot` runs the `cloud_browse` example (`examples/cloud_browse/`) on a private Xvfb screen and saves a PNG of its window. From the repo root, one xdotool step per argument, `$WID` the window:

```sh
EXAMPLE=cloud_browse WAIT=0 agent-screenshot temp/subtle.png 'sleep 2' 'mousemove --window $WID 155 73' 'sleep 0.5' 'click 1' 'sleep 0.8' 'mousemove --window $WID 115 165' 'sleep 0.3' 'click 1' 'sleep 0.8'
```

The window is 1300×760 at root (0,0), so PNG pixels equal `--window $WID` coordinates. Timing that works: `sleep 2` after launch, `sleep 0.5` before each click, `sleep 0.8` after (`WAIT=0`, sleeps as steps). The demo starts in Syncative dark with Zebra off. The pick list ignores keys, so click and never use arrows or Return.

Item centres, measured from the pixels and hit 6 of 6 times in one try each (open the menu, then click the entry's centre):

- Theme: opens at `(155, 73)`; Default light `(110, 100)`, Default dark `(110, 128)`, Syncative light `(110, 156)`, Syncative dark `(110, 184)`
- Zebra: opens at `(115, 106)` in every theme; off `(115, 136)`, subtle `(115, 165)`, strong `(115, 193)` — Syncative items are 28 px tall, the Default themes' about 31 px
- Action: opens at `(840, 106)`; under list `(840, 135)`, floating `(840, 163)`, toolbar `(840, 191)`, none `(840, 219)` — with "none" the demo skips `Cabinet::action` and no button is drawn (the greyed wide button under the list disappears)
- Toolbar icons at `y ≈ 174`: the leftmost icon `44` toggles browse/tree mode (tree shows the root row `My files` with carets); on the right, Create folder `1109`, Copy `1141`, Paste `1173`, Duplicate `1205`, Delete `1252`
- List checkboxes at `x = 49`, rows 37 px apart: header (selects all) `y = 215`, Dokumente `252`, Finanzen `289`, Projekte `326`, readme.md `363`; the topmost checkbox band in a column scan is the header's, not the first row's
- Confirmation/new-folder modal card spans `x 500–799`; its accent confirm button is centred at `(750, 492)` for the four-line delete card (title, question, count, buttons) and one line-gap higher for three-line cards (scan the accent band down column `x = 750` to re-measure after any card change)

The `picker` example (`EXAMPLE=picker`, window 900×620, Default dark) opens its pickers from the Browse buttons at `x = 828`: Folder `y = 84`, Excluded `133`, Hidden `182`. In the open picker card (`x 140–759`) the row checkboxes sit at `x = 166`, rows 36 px apart from `y = 214`, row names from `x ≈ 220`; the footer's confirm button is centred at `(720, 529)` for "Save" and `(683, 529)` for "Use this folder", Cancel at `(645, 529)` and `(570, 529)`.

For a target that is not listed, take the shot with `GRID=50` and no steps, then measure its colour bands with `SCAN=300` (or `agent-screenshot --scan temp/shot.png 300` on a shot taken without `GRID`): each band's `y` range and RGB down column 300, `SCAN=y=178` for a row.

## Verifying zebra stripes against the header

`stripe_of` in `src/cabinet/paint.rs` bounds the stripe by the header colour. Scanning column 300 of a cabinet shot gives background, header band, then the alternating row bands; background / header / stripe should come out as:

- light themes, strong: stripe equals the header, never darker (Syncative light 255 / 235 / 236, Default light 255 / 240 / 240)
- light themes, subtle: strictly lighter than the header, about 5–6 levels below the background (Syncative light 255 / 235 / 249, Default light 255 / 240 / 250)
- Syncative dark: the full factor, its header is 40 % off the background (subtle 42 / 24 / 37, strong 42 / 24 / 27)

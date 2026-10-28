# Notes for coding agents

Read [doc/README.md](doc/README.md) and the page matching the task before you start: conventions, architecture and research results are there. Keep what you learn about this project in `doc/`, not in an agent's memory.

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
- List checkboxes at `x = 49`, rows 37 px apart: header (selects all) `y = 219`, Dokumente `256`, Finanzen `293`, Projekte `330`, readme.md `367`; the topmost checkbox band in a column scan is the header's, not the first row's

For a target that is not listed, take the shot with `GRID=50` and no steps, then measure its colour bands with `SCAN=300` (or `agent-screenshot --scan temp/shot.png 300` on a shot taken without `GRID`): each band's `y` range and RGB down column 300, `SCAN=y=178` for a row.

## Verifying zebra stripes against the header

`stripe_of` in `src/cabinet/paint.rs` bounds the stripe by the header colour. Scanning column 300 of a cabinet shot gives background, header band, then the alternating row bands; background / header / stripe should come out as:

- light themes, strong: stripe equals the header, never darker (Syncative light 255 / 235 / 236, Default light 255 / 240 / 240)
- light themes, subtle: strictly lighter than the header, about 5–6 levels below the background (Syncative light 255 / 235 / 249, Default light 255 / 240 / 250)
- Syncative dark: the full factor, its header is 40 % off the background (subtle 42 / 24 / 37, strong 42 / 24 / 27)

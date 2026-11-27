# solidcraft-woodwork

A woodworking plugin for SolidCraft: parametric cabinets, joinery, materials, shop drawings
and a cut list. The next version needs SolidCraft 0.50 or later (0.5.0 needs 0.49); for older SolidCraft, see
[SolidCraft versions](#solidcraft-versions). The roadmap is in [PLAN.md](PLAN.md).

![Woodwork, a SolidCraft plugin: parametric cabinets, joinery and cut lists](docs/banner.png)

[![Watch the one minute introduction on YouTube](docs/intro-video.jpg)](https://www.youtube.com/watch?v=N4oWhdVGvVY)

The picture opens a one minute introduction on YouTube. Three how-tos follow it in the
[Woodwork for SolidCraft playlist](https://www.youtube.com/playlist?list=PLCpxga1LxgoA).

## Functions

Under **Plugins > Woodwork**, and as buttons on the ribbon:

- **Cabinet**: makes a cabinet, or changes one you already made. Its **Mode** starts on Auto:
  pick a door, drawer front, shelf, divider or drawer part and it edits that cabinet; pick a
  side, top, bottom or back and it makes a new one next to that cabinet; pick nothing for a
  new one on its own.
  - A box cabinet with shelves, dividers, a back and a plinth / toe kick (a board, notched
    sides, legs or casters).
  - Doors, drawers, or drawers above or below doors, overlay or inset, that open on their
    joints. Holes for shelf pins, handles, hinges and drawer slides, or no handles at all:
    a grip edge bevelled behind the front, a finger groove along it, or a tab under a drawer
    front to pull by.
  - Fronts flat, Shaker (a frame round a panel), glass doors in a Shaker frame, or with
    vertical or horizontal grooves, and drawer fronts separate or integrated (the drawer
    box's own front is the front). Where overlay doors meet on a divider: half overlay
    hinges, or double dividers so every door gets standard hinges.
  - A sink base (rails instead of a top), and corner cabinets: L-shaped with a pair of
    folding doors, or diagonal with one door.
  - Hangs on the wall on a French cleat, a hanging rail or a steel rail.
  - Placed against the cabinet you picked, and held to it, so a run stays together when a
    cabinet changes size. Or along a line: a sketch line, a wall's edge or face, as wide as
    the line if you like. Match selected puts the cabinets of a run on one shared height,
    depth, thickness and plinth, so changing one parameter resizes the whole run.
- **Woodwork settings**: what is the same for the whole design, a dialog per group: General
  (board material and thickness), Doors and fronts, Drawers, Plinth / toe kick and wall mount,
  Joinery, Cut list and worktop, and Display. On the Model tab they are under one **Woodwork
  settings** button with a dropdown; each is also in the menu. New cabinets start from them.
- **Apply settings**: builds the picked cabinets, or all of them, again with the Woodwork
  settings, keeping each cabinet's own sizes and choices. It runs as a long job with progress
  per cabinet and a Cancel button.
- **Joinery** (in Woodwork settings): dowels, finger joints, confirmat screws, plain screws,
  pocket screws, cam locks, biscuits or a dado, drilled as the cabinet is made and counted in
  the cut list. Finger joints also for the drawer boxes.
- **Material**: birch plywood, MDF, white melamine or oak, for a whole cabinet or just the
  parts you pick (the doors, say).
- **Panel**: an end panel or a filler on one side of a cabinet, following its size.
- **Mobile base**: one frame on locking casters under a group of cabinets, bolted to each
  other and, when made without a plinth, screwed down to the frame, for a workbench that
  rolls.
- **Worktop**: over the picked cabinets, with butt or mitred corners, dog holes, a sink
  cut-out, an opening for a benchtop tool set flush with the top (its shelf drilled for
  the tool's bolts), and a side wing on two legs past one end.
- **Drawing**: an A3 shop drawing per cabinet, with front, right and iso views, its sizes and
  its boards.
- **Cut list**: every board with its size, edge banding and grain, the hardware to buy, a cost
  estimate from your prices, and
  cutting layouts for the sheets (saw kerf, trim, grain). As a printable PDF with a cutting
  diagram per sheet, CSV, part labels for A4 sticker sheets, or a DXF per sheet for a CNC.

Every size is a design parameter: change one in Parameters and the cabinet follows.

**[How to use each function](docs/usage.md)**

## Install

1. Download `woodwork-<version>.zip` from the [latest release](../../releases/latest)
   (under **Assets**, not the source code zip).
   To try new functions early, take the newest release candidate (marked Pre-release) from
   the [releases page](../../releases) instead. It may still change before the full release.
2. In SolidCraft open **Preferences > Plugins** and choose **Install plugin...**

Or find **Woodwork** in the SolidCraft Workshop and install it from there.

The functions then appear under **Plugins > Woodwork**.

## SolidCraft versions

| SolidCraft | Plugin | Notes |
|---|---|---|
| 0.50+ | next | Tested on 0.50.0. Cabinets placed along a wall or sketch line; Mobile base with connector bolts and frame screws; handle-less fronts (grip edge, finger groove, bottom tab), Shaker, glass and grooved fronts, and integrated drawer fronts; half overlay hinges or double dividers where doors meet on a divider; a cost estimate in the cut list; Match selected as one shared group per run; the worktop's side wing and tool bolt holes; Woodwork settings as a dialog per group under one ribbon dropdown; and links from the dialogs' fields to the usage guide. |
| 0.49 | 0.5.0 | Tested on 0.49.0. One Cabinet dialog that makes or edits (Auto mode), Woodwork settings in sections, Apply settings as a long job, and finger joints. |
| 0.47 | 0.4.0 | Tested on 0.47.0; works on 0.46 too. Joinery and hardware are design-wide Woodwork settings, one dialog for box and corner cabinets, and hinges, slides and fasteners as models. |
| 0.46 | 0.3.0 | Tested on 0.46.1. Dialogs show only the fields that apply, Woodwork settings give new cabinets their defaults. |
| 0.45 | 0.2.0 | Tested on 0.45.0. The view is framed after a cabinet is added, and agents get each function as a tool. |
| 0.44 | 0.1.8 | A cabinet is previewed before it is added, and each function has its own icon. |
| 0.43 | 0.1.6 and 0.1.7 | Dropdowns explain each choice, and installing asks you to approve the plugin. |
| 0.42 | 0.1.4 and 0.1.5 | A button per function on the ribbon. |
| 0.41 | 0.1.0 to 0.1.5 | The first SolidCraft with plugins. 0.1.4 and 0.1.5 were not tested on it. |
| 0.40 and earlier | none | No plugin support. |

A design made with the plugin is ordinary bodies, joints and parameters: it opens in
SolidCraft with or without the plugin.

## Example

The kitchen in the banner is in the repo:

- [`examples/kitchen.sc`](examples/kitchen.sc): the design. Open it in SolidCraft; it needs
  no plugin.
- [`examples/kitchen-cutlist.csv`](examples/kitchen-cutlist.csv): its cut list, 140 boards.
- [`examples/kitchen.py`](examples/kitchen.py): the script that built both. An L-shaped
  kitchen with a tall unit, base and wall units placed against each other and matched in size
  (change `Base_1_height` and the floor run follows), an L-shaped and a diagonal corner
  cabinet, a sink base, an inset door, white melamine carcasses with oak fronts, end panels,
  an oak worktop with a sink cut-out, a different joinery method on several cabinets, a shelf
  added with an Edit, a shop drawing of every cabinet, and some doors and a drawer left open.

To rebuild it, run `python build.py` and then `python examples/kitchen.py`.

## Build from source

Needs Rust with the `wasm32-unknown-unknown` target, `wasm-tools`, Python and the
`solidcraft-cli` that comes with SolidCraft.

    python build.py            # unit tests, build, package, end-to-end tests
    python build.py --install  # the same, then install into SolidCraft

The package lands in `dist/woodwork-<version>.zip`; it can also be installed from
**Preferences > Plugins > Install plugin...**

## Layout

- `plugin.json`: the manifest: the functions, their dialogs and the app commands they use.
- `src/lib.rs`: runs each function and talks to the app.
- `src/cabinet/`, `src/corner/`, `src/edit/` (Cabinet in Edit mode), `src/panel/`, `src/worktop/`, `src/settings/`,
  `src/material.rs`, `src/cutlist/`, `src/drawing/`: one module per function, plain Rust.
  The cut list's files are written by `src/pdf/` and `src/dxf/`, its cutting layouts by
  `src/nesting/`.
- `src/kit/`, `src/fittings.rs`, `src/joint/` and `src/hardware/`: what the cabinets share:
  panels and parameters, hardware, joinery, and the hinge and slide sizes.
- `src/select/`: finds the picked cabinet.
- `build.py`: build, package and end-to-end tests through `solidcraft-cli`.
- `tests/`: the end-to-end tests. Unit tests are in `tests.rs` beside each module.

## Licence

MIT, see [LICENSE](LICENSE). `wit/plugin.wit` is SolidCraft's plugin interface, from
[tangency-labs/solidcraft](https://github.com/tangency-labs/solidcraft) (MIT OR Apache-2.0).

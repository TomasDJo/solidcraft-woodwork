# solidcraft-woodwork

A woodworking plugin for SolidCraft: parametric cabinets, joinery, materials, shop drawings
and a cut list. Version 0.5.0 needs SolidCraft 0.49 or later; for older SolidCraft, see
[SolidCraft versions](#solidcraft-versions). The roadmap is in [PLAN.md](PLAN.md).

![Woodwork, a SolidCraft plugin: parametric cabinets, joinery and cut lists](docs/banner.png)

[![Watch the one minute introduction on YouTube](docs/intro-video.jpg)](https://www.youtube.com/watch?v=N4oWhdVGvVY)

The picture opens a one minute introduction on YouTube. Three how-tos follow it in the
[Woodwork for SolidCraft playlist](https://www.youtube.com/playlist?list=PLCpxga1LxgoA).

## Functions

Under **Plugins > Woodwork**, and as buttons on the ribbon:

- **Cabinet**: makes a cabinet, or changes one you already made. Its **Mode** starts on Auto:
  pick a door, drawer front, shelf, divider or drawer part and it edits that cabinet; pick a
  side, top, bottom or back, or nothing, and it makes a new one (next to the picked one).
  - A box cabinet with shelves, dividers, a back and a plinth / toe kick (a board, notched
    sides, legs or casters).
  - Doors, drawers, or drawers above or below doors, overlay or inset, that open on their
    joints. Holes for shelf pins, handles, hinges and drawer slides.
  - A sink base (rails instead of a top), and corner cabinets: L-shaped with a pair of
    folding doors, or diagonal with one door.
  - Hangs on the wall on a French cleat, a hanging rail or a steel rail.
  - Placed against the cabinet you picked, and held to it, so a run stays together when a
    cabinet changes size. Match selected takes the picked cabinet's sizes.
- **Woodwork settings**: what is the same for the whole design, in sections: board material
  and thickness, doors and fronts, drawers and slides, plinth and wall mount, joinery, cut
  list and worktop, and display. New cabinets start from them.
- **Apply settings**: builds the picked cabinets, or all of them, again with the Woodwork
  settings, keeping each cabinet's own sizes and choices. It runs as a long job with progress
  per cabinet and a Cancel button.
- **Joinery** (in Woodwork settings): dowels, finger joints, confirmat screws, plain screws,
  pocket screws, cam locks, biscuits or a dado, drilled as the cabinet is made and counted in
  the cut list. Finger joints also for the drawer boxes.
- **Material**: birch plywood, MDF, white melamine or oak, for a whole cabinet or just the
  parts you pick (the doors, say).
- **Panel**: an end panel or a filler on one side of a cabinet, following its size.
- **Worktop**: over the picked cabinets, with butt or mitred corners, dog holes, a sink
  cut-out, and an opening for a benchtop tool set flush with the top.
- **Drawing**: an A3 shop drawing per cabinet, with front, right and iso views, its sizes and
  its boards.
- **Cut list**: every board with its size, edge banding and grain, the hardware to buy, and
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

The functions then appear under **Plugins > Woodwork**.

## SolidCraft versions

| SolidCraft | Plugin | Notes |
|---|---|---|
| 0.49+ | 0.5.0 | Tested on 0.49.0. One Cabinet dialog that makes or edits (Auto mode), Woodwork settings in sections, Apply settings as a long job, and finger joints. |
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
- `src/cabinet/`, `src/corner/`, `src/edit/`, `src/panel/`, `src/worktop/`, `src/settings/`,
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

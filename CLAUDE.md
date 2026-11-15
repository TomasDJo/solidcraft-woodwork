# solidcraft-woodwork

A plugin for SolidCraft (0.41+) that turns it into a woodworking tool: parametric cabinets,
materials, joinery and cut lists.
`README.md` is the user-facing description, `PLAN.md` the roadmap with what is done and open.

## How SolidCraft plugins work

- A plugin is a folder with `plugin.json` and one WebAssembly **component**. It exports
  `execute(entry, params_json)` and can only call the app's ordinary commands through
  `commands.run` (the same commands MCP and `solidcraft-cli` use). Interface: `wit/plugin.wit`.
- The manifest lists the entries, a JSON schema per entry, and every command id the plugin
  calls. A command that is not listed is refused. Entry names: lowercase letters, digits,
  hyphens (no underscores).
- The app draws each entry's dialog from its schema and shows the entry under the Plugins menu
  (0.42: a Woodwork submenu, and one ribbon button per entry, labelled by its `ribbon` slot).
  A schema keyword the dialog cannot draw or check (`pattern`, `readOnly`, ...) removes the
  entry from the menu in 0.42; ours use only type, title, description, default, enum, bounds
  and lengths.
  The dialog is **static**: no dependent fields, no stored values, no preview, no pick-a-body
  field. Plugin code runs only after OK.
- One run is one undo step. A failed command discards the whole run. The result is ordinary
  features, so a design opens without the plugin.
- Limits per run: 256 app commands, 30 s, 1 MiB result. No file or network access (file grants
  exist but the app has no way to give them yet), no storage.
- A `read` entry may only call read commands. `selection.get`, `component.list` and
  `assembly.mass_properties` count as edits; `document.inspect` (which also returns the
  selection), `inspect.measure` and `model.mesh` are read.
- Inside a plugin run, `solid.hole` checks its `position` in the active component's own
  coordinates but stores it shifted by where that component stands in the world, so the hole
  misses on a component that has been moved (through the CLI directly it does not). Joinery
  therefore moves the cabinet to the origin, drills, and moves it back.
- A pattern of a hole can lose copies ("cut copies remove no material") when the component has
  been moved and another body stands where its local numbers point in the world. Joinery makes
  every hole a feature of its own for that reason; the shelf pin patterns have not shown it.
- The plugin runs on a background copy of the design. Display commands such as `view.fit` are
  accepted but do nothing in the window.
- Official docs: `docs/plugins.md`, `docs/plugin-runtime.md` and `docs/rfcs/0002-plugins.md` in
  `tangency-labs/solidcraft`. A `ui` and a `storage` capability are planned there.

## Layout

| File | What |
|---|---|
| `plugin.json` | Manifest: entries, dialog schemas, allowed commands, version |
| `src/lib.rs` | Entry dispatch (`execute`) and the wasm guest. The only place that runs commands |
| `src/kit.rs` | Shared by the cabinet builders: size fields, design parameters, panels, moving parts on joints, history folder, summary |
| `src/cabinet.rs` | Box cabinet builder: parameters in, list of commands out |
| `src/corner.rs` | L-shaped corner cabinet builder, written for a left corner and mirrored for a right one |
| `src/joinery.rs` | Screw and dowel holes for an existing cabinet |
| `src/material.rs` | Board materials: create on first use, assign |
| `src/cutlist.rs` | Cut list and sheet estimate from `inspect.measure` |
| `src/select.rs` | Finds the selected cabinet and its parts by body name |
| `build.py` | Unit tests, wasm build, package, end-to-end tests, optional install |
| `examples/kitchen.py` | Builds the sample kitchen, `examples/kitchen.sc` and its cut list CSV, through the CLI. Rerun it and re-render `docs/kitchen.png` when the plugin's output changes |

The pattern: a module is plain Rust that turns input into `Vec<(command, json args)>`, so
`cargo test` covers it without SolidCraft. `lib.rs` runs the list. Anything that needs a reply
from the app first (selection, measurements, the active component) is fetched in `lib.rs` and
passed in.

Entries today: `cabinet`, `corner`, `joinery`, `material` (all edit) and `cutlist` (read).

A new kind of cabinet is a new module that builds on `Kit` and a new entry with its own dialog.
`lib.rs` runs `cabinet` and `corner` through the same steps (component, material, place,
ground), so a builder only returns its commands and `Kit`'s summary.

## Build, test, install

    python build.py            # cargo test, wasm build, package, end-to-end tests
    python build.py --install  # the same, then install into SolidCraft
    python examples/kitchen.py out.sc

- Needs Rust with the `wasm32-unknown-unknown` target and `wasm-tools` (both in `~/.cargo/bin`,
  which is not on PATH in a fresh shell; `build.py` adds it).
- The end-to-end tests in `build.py` run the built plugin through `solidcraft-cli` and check
  real geometry (bounding boxes, hole counts, joint moves). A new feature gets a case there.
- `solidcraft-cli snapshot --in file.sc --out shot.png` renders a result; look at it before
  calling a geometry change done.
- Output goes to `dist/` (ignored by git).

## Rules

- **Versions move slowly.** Bump only the last digit, in both `plugin.json` and `Cargo.toml`.
  0.1.0 was the first public release.
- **CI must stay green.** `.github/workflows/build.yml` runs `cargo fmt --check`, clippy with
  warnings as errors, and `python build.py` (unit and end-to-end tests against a downloaded
  SolidCraft CLI) on every push and pull request. Run `cargo fmt` before committing.
- **The sample kitchen shows everything.** A new function goes into `examples/kitchen.py` in
  the same branch; rerun it (it rewrites `examples/kitchen.sc` and the cut list CSV) and
  re-render `docs/kitchen.png`:
  `solidcraft-cli snapshot --in examples/kitchen.sc --out docs/kitchen.png --camera '{"target":[1800,-350,1000],"yaw":0.22,"pitch":0.33,"distance":5600,"fov":0.6}' --no-grid --no-axes --width 1400 --height 900 --quality fine`
- **Releases come from tags.** Pushing a tag `vX.Y.Z` that matches the version in
  `plugin.json` makes the same workflow attach `woodwork-X.Y.Z.zip` to a GitHub Release.
- **Branches and pull requests, never straight to main.** One branch per function, one commit
  per function, with `README.md` updated and `PLAN.md` ticked in the same commit. Do not push
  until the user says so.
- **Every length sent to the app carries its unit** (`"582 mm"`). A bare number in a size field
  is read in the design's unit, so an inch design would get 582 inches. Exceptions that take
  plain mm numbers only: `occurrence.move` `translate` and a joint's `point`.
- **Cabinets are parametric.** Sizes become design parameters named `<Prefix>_width` and so on
  (`cabinet::prefix` makes the name an identifier). Every panel and hole is an expression over
  them. The numeric values in `cabinet.rs` are for validating input only. Counts (shelves,
  dividers, doors, drawers) are fixed when the cabinet is built.
- **A cabinet is a component**, created inside whichever component is active, and the active
  component is restored afterwards. **A part that moves is a sub-component with a joint**
  (doors: revolute, drawers: slider). New moving parts follow the same rule. A part can hang
  on another moving part: a corner cabinet's second door is jointed to the first (`Door return
  fold`, or `Door fold` when the pair hangs on the return), not to the cabinet, so it follows it.
- **Body names are the data model.** Bodies are `<Cabinet> <Part>`; a cabinet is recognised by
  its `<Cabinet> Side L` body, and the longest matching cabinet name owns a body. Never name
  another part with the suffix ` Side L` (drawer box sides are `Left` and `Right` for that
  reason). Two cabinets cannot share a name: `kit::free_name` gives a taken name the next free number.
- **Geometry convention:** X width, Y depth with the front at -Y, Z up. A cabinet is centred on
  its component origin and stands on Z = 0.
- **Features made at cabinet level go into one history folder** (`timeline.group`).
- Keep code minimal: no abstractions for one use, no new dependencies (only `serde_json` and
  `wit-bindgen`). A deliberate shortcut gets a `ponytail:` comment naming its limit.
- Hardware numbers (32 mm system, 35 mm cups, 12.7 mm slides, 128 mm pulls, confirmat and dowel sizes) are
  common-practice values, not taken from a manufacturer's sheet.

## Working with the running app

- The MCP server `solidcraft` talks to the running app (`solidcraft-live` is on a wrong port;
  ignore it). The user watches the window, and their open design may be unsaved.
- `new_design` and `open` replace the open design. Do not use them without asking. To test in
  the open design, make one plugin run and `undo` it, and say so.
- MCP may only read a few folders. To run a development build through MCP, copy
  `dist/woodwork/` into the `solidcraft-mcp-*` temp folder the server names in its errors,
  then `plugin.inspect` (gives the approval hash) and `plugin.run`.
- For anything bigger, build into a file with the CLI (as `examples/kitchen.py` does) and let
  the user open it.
- On this machine, Rust code inside a bash heredoc breaks the shell quoting. Write and edit
  source files with the file tools, not `cat <<EOF`.

## Known limits

- The dialog cannot remember values or adapt to the Units choice: defaults are always mm.
- The camera is not framed after a run; the user presses Fit.
- `select.rs` and cabinet placement (`place`) read world bounding boxes, so they are wrong for
  a cabinet that is rotated off the axes.
- The cut list measures a board by its world bounding box when the board fills that box, and
  otherwise (an open door, a turned cabinet) by `model.mesh` in the body's own component: one
  extra command per such board, so about 250 of them at most.
- A cabinet with doors or drawers cannot be made inside a component that has been moved:
  the hinge cup holes fail with "not on a face of a body" (the shift described above).
- Joints use a fixed point from creation time. They do not follow a resize, and they are off
  if the cabinet is made inside a component that has already been moved.
- A cabinet is grounded at the end of a run, because a joint only moves against something
  fixed. Moving it later means ungrounding it first.
- The hardware list is counted from feature and body names (`hinge cup`, `handle`, `pins`,
  `joint`, `Drawer n Front`), so renaming those breaks it.
- The sheet estimate is area plus 15 % waste on 2440 x 1220, not a cutting layout.
- A corner cabinet's two end panels are named `Side L` and `Side R`, so selection and placing
  see its whole footprint. Joinery refuses it (it has a `<Prefix>_return` parameter). A box
  cabinet can be placed beside its back leg, but not turned to continue its return.
- Match selected links a new cabinet's parameters to the selected cabinet's (`B_height =
  A_height`; `kit::matched` passes them to `Kit` as `_match`). Deleting the first cabinet's
  parameters breaks the ones linked to it, and a matched cabinet gets a plinth only if the
  selected one has one. Material is not matched.
- Joinery covers the four carcass corners only, and tells an overlaid back from a grooved one
  by comparing the side depth with the depth parameter.
- Parts are modelled as an exact fit, with no clearance for the back panel or shelves.
- Functions that use the selection (Place, Material, Joinery) were tested through MCP and the
  CLI only. Whether the selection survives opening the Plugins menu dialog has not been
  confirmed in the app.

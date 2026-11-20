# solidcraft-woodwork

A plugin for SolidCraft (0.45+) that turns it into a woodworking tool: parametric cabinets,
materials, joinery and cut lists.
`README.md` is the short user-facing description, `docs/usage.md` how to use each function,
`PLAN.md` the roadmap with what is done and open.

## How SolidCraft plugins work

- A plugin is a folder with `plugin.json` and one WebAssembly **component**. It exports
  `execute(entry, params_json)` and can only call the app's ordinary commands through
  `commands.run` (the same commands MCP and `solidcraft-cli` use). Interface: `wit/plugin.wit`.
- The manifest lists the entries, a JSON schema per entry, and every command id the plugin
  calls. A command that is not listed is refused. Entry names: lowercase letters, digits,
  hyphens (no underscores).
- The app draws each entry's dialog from its schema and shows the entry under the Plugins menu
  (a Woodwork submenu, and one button per entry in a Woodwork group on the Plugins ribbon tab;
  a `ribbon` slot named after a host tab such as Model would put it there instead).
  A schema keyword the dialog cannot draw or check (`pattern`, `readOnly`, ...) leaves the
  entry in the menu with Run turned off; ours use only type, title, description, default,
  oneOf, bounds and lengths. The dialog shows the fields that `required` names first, in that
  order, and the rest by key: so every schema lists all its fields in `required`, which also
  puts a ` *` after each label. `build.py` packs the manifest without white space: the app
  refuses one over 64 KiB. A choice is a `oneOf` list of `{const, title, description}` (0.43; `enum` shows the
  bare values, and 0.42 cannot draw `oneOf`). `tests/test_manifest.py` checks all of this.
  The dialog is **static**: no dependent fields, no stored values.
- **Tools for agents (0.45).** Every entry is also the tool `woodwork.<entry>` (`plugin.tools`,
  MCP `tools/list`), with its schema as the arguments. Over MCP the arguments are checked
  strictly first: a field `required` names must be there even when it has a default, and
  undeclared fields are refused. As every field is required (for the dialog's order), an agent
  passes them all; the error names a missing one. The `woodwork.<entry>` command in a script
  (`approval` and `params`) and `plugin.run` pass the fields through unchecked.
  `tests/test_tools.py` drives `solidcraft-cli mcp --headless` for this.
- **Preview (0.44).** While the dialog is open the app runs an edit entry as a dry run on a
  copy of the design each time a field changes, and draws the result. So plugin code runs
  before OK, without folder grants; nothing a run does may depend on being the only run.
- **The selection is a field (0.44).** Every entry that works on the selected cabinet has a
  `selection` array (`maxItems: 1`; Material takes several). The app reserves that name for
  bodies, faces and edges: it draws a pick control, fills it with what is selected when the
  dialog opens, and sends selection references. (`x-solidcraft-selection` on a field of any
  name does the same and costs 70 bytes of manifest a field; the reserved `bodies` takes
  bodies only, and a click on a board selects a face.) `select::picked`
  reads the field and falls back to the design's selection when it is empty, which is what
  scripts and the tests mostly use. Any other array, or more keywords on it, turns Run off. Edit skips the key (`edit::merge`), and a cabinet
  does not remember it.
- One run is one undo step. A failed command discards the whole run. The result is ordinary
  features, so a design opens without the plugin.
- Limits per run: 256 app commands, 30 s, 1 MiB result. No network access. The only plugin
  data we keep in the design is the prices (`storage@0.1`, below).
  Files: only `files.write-file` into a folder the manifest asks for under
  `runtime.permissions.files` and the user approves when installing (0.43: the install
  screen asks for a path). The plugin names the grant and a relative path and never learns
  where the folder is. A refused write fails the whole run, so the cut list only writes when
  its Save as field is filled. A `read` entry may not write files from 0.45 on, so `cutlist` is
  an `edit` entry though it changes nothing. Through the CLI a grant comes
  from `plugin.install` with `approve` earlier in the same script; `tests/test_cutlist.py`
  does that in a profile folder of its own (`LOCALAPPDATA` and friends pointed at `dist/`).
- A `read` entry (we have none now) may only call read commands. `selection.get`, `component.list` and
  `assembly.mass_properties` count as edits; `document.inspect` (which also returns the
  selection), `inspect.measure` and `model.mesh` are read.
- Inside a plugin run, `solid.hole` checks its `position` in the active component's own
  coordinates but stores it shifted by where that component stands in the world, so the hole
  misses on a component that has been moved (through the CLI directly it does not). Joinery
  therefore moves the cabinet to the origin, drills, and moves it back. (The plugin now cuts
  cylinders instead of holes, see Rules. Whether a cylinder cut is shifted the same way has
  not been tried, so Joinery still does this.)
- A pattern of a hole can lose copies ("cut copies remove no material") when the component has
  been moved and another body stands where its local numbers point in the world. Joinery makes
  every hole a feature of its own for that reason; the shelf pin patterns have not shown it.
- The plugin runs on a background copy of the design. From 0.45 the window applies the last
  view command of an accepted run (`view.fit`, `view.home`); the preview's dry runs leave the
  camera alone. `execute` ends a cabinet, corner or edit run with `select.clear` and
  `view.fit`: Fit frames the selection when there is one, such as the board a cabinet was placed
  against. `plugin.run` through the CLI reports it as `view: {kind: fit}`.
- `build.py` runs `solidcraft-cli plugin check` on the package: an error fails the build.
- Official docs: `docs/plugins.md`, `docs/plugin-runtime.md` and `docs/rfcs/0002-plugins.md` in
  `tangency-labs/solidcraft`. A `ui` capability is planned there.
- **Plugin data in a design (`storage@0.1`, 0.45; "Plugin data in a design" in
  `docs/plugins.md`).** We keep one small JSON document, the prices, written by `prices` with
  `storage.patch` (an RFC 7396 merge patch of only the filled fields) and read by `prices` and
  `cutlist` with `storage.get` (`document` is null in a design without our data):
  `{"currency", "sheet": {<material>: price}, "banding": {"0.4"|"1"|"2": price},
  "worktop": price, "hardware": {<item as cutlist::hardware names it>: price}}`. The manifest
  has the capability, the two commands and `runtime.data_format: 1`; a change to the document's
  shape that older data cannot be read as raises it, and `storage.get`'s `format` lower than
  `plugin_format` is then data to convert. The data is saved in the `.sc` and survives without
  the plugin. Writing works only inside a run (a script reads it with `storage.get {plugin:
  "woodwork"}`); installing with `approve: {command_groups: [], folders: []}` still works.

## Layout

| File | What |
|---|---|
| `plugin.json` | Manifest: entries, dialog schemas, allowed commands, version |
| `src/lib.rs` | Entry dispatch (`execute`) and the wasm guest. The only place that runs commands |
| `src/<module>/mod.rs`, `tests.rs` | Each module below that has unit tests is a folder: the code in `mod.rs`, its tests in `tests.rs` beside it |
| `src/kit/` | Shared by the cabinet builders: size fields, design parameters, panels, moving parts on joints, history folder, summary |
| `src/fittings.rs` | Hardware both cabinet builders use: the constants, shelf pin rows, hinge cup heights, handle height and screw holes |
| `src/cabinet/` | Box cabinet builder: parameters in, list of commands out. `mod.rs` is the carcass and the order of things, `input.rs` reads and checks the fields, `doors.rs` and `drawers.rs` are the fronts |
| `src/corner/` | Corner cabinet builder, L-shaped or diagonal, written for a left corner and mirrored for a right one. `mod.rs` is the carcass, `input.rs` reads the dialog, `doors.rs` the L's bi-fold pair, `diagonal.rs` the diagonal front's door and plinth |
| `src/edit/` | Edit cabinet: what a cabinet's component remembers (`component.metadata.set` under our key), and merging that with the Edit dialog and the design's parameters |
| `src/joinery/` | Screw and dowel holes for an existing cabinet |
| `src/worktop/` | Worktop over a straight run of the selected cabinets: finds the run from the cabinets' sides and backs, and builds one board whose parameters are expressions over the cabinets'. `lib.rs` makes its component, moves it to the run's fixed point (a grounded cabinet's middle, or the side held to a cabinet outside the run) and grounds it |
| `src/material.rs` | Board materials: create on first use, assign |
| `src/cutlist/` | Cut list, edge banding and grain from `inspect.measure`, and the sheets to buy from the cutting layouts |
| `src/nesting/` | Cutting layouts: parts packed onto sheets per material and thickness, with kerf, trim and grain, in guillotine cuts a panel saw can make |
| `src/pdf/` | The cut list as an A4 PDF, written by hand with the viewer's own Helvetica (WinAnsi text, no embedded font), with a cutting diagram per sheet |
| `src/dxf/` | The cutting layouts as DXF R12, a file per sheet, written by hand: sheet, part outlines and names, and holes (read with `model.faces`, mapped onto the part as it lies on the sheet) on a layer per diameter and depth |
| `src/select/` | Finds the selected cabinet and its parts by body name |
| `src/prices/` | Prices: the merge patch from the Prices dialog, and the cut list's cost estimate from the stored prices (reply, CSV; the PDF draws it) |
| `icons/*.svg` | One icon a function, named in its input schema (`x-solidcraft-icon`), shown on the ribbon, in the menu and in the dialog. In the app's icon subset: `viewBox="0 0 64 64"`, absolute `M L C Q A Z` paths, colours from its icon palette only (`crates/look/icons/design/palette.toml` in `tangency-labs/solidcraft`), strokes with round joins and caps. The app leaves out an icon that breaks a rule, without a message |
| `build.py` | Unit tests, wasm build, package, runs the end-to-end tests, optional install |
| `tests/test_*.py` | End-to-end tests, one file per function of the plugin; every `test_*(approval)` function in them is run. `tests/helpers.py` has the shared steps and readers |
| `docs/intro-video.jpg` | The introduction video's thumbnail with a play button, the README's second picture; it links to the video on YouTube |
| `docs/banner.py` | Draws `docs/banner.png`, the README's first picture (plugin lists show it): a small run of cabinets on a dark background beside the name. Needs Pillow. Rerun it when the cabinets look different |
| `examples/kitchen.py` | Builds the sample kitchen, `examples/kitchen.sc` and its cut list CSV, through the CLI. Rerun it when the plugin's output changes |

The pattern: a module is plain Rust that turns input into `Vec<(command, json args)>`, so
`cargo test` covers it without SolidCraft. `lib.rs` runs the list. Anything that needs a reply
from the app first (selection, measurements, the active component) is fetched in `lib.rs` and
passed in.

Entries today: `cabinet`, `corner`, `edit`, `joinery`, `material`, `worktop`, `cutlist` and `prices`, all edit.

A new kind of cabinet is a new module that builds on `Kit` and a new entry with its own dialog.
`lib.rs` runs `cabinet` and `corner` through the same steps (component, material, place,
ground), so a builder only returns its commands and `Kit`'s summary.

## Build, test, install

    python build.py            # cargo test, wasm build, package, end-to-end tests
    python build.py corner     # the same, but only the end-to-end tests named like that
    python build.py --install  # the same, then install into SolidCraft
    python examples/kitchen.py out.sc

- Needs Rust with the `wasm32-unknown-unknown` target and `wasm-tools` (both in `~/.cargo/bin`,
  which is not on PATH in a fresh shell; `build.py` adds it).
- The end-to-end tests in `tests/` run the built plugin through `solidcraft-cli` and check
  real geometry (bounding boxes, hole counts, joint moves). A new feature gets a case there.
- `solidcraft-cli snapshot --in file.sc --out shot.png` renders a result; look at it before
  calling a geometry change done.
- Output goes to `dist/` (ignored by git).

## Rules

- **Versions move slowly.** Bump only the last digit, in both `plugin.json` and `Cargo.toml`.
  0.1.0 was the first public release.
- **Every fix and every function has a test that fails without it.** A bug gets a regression
  test in the same commit as its fix: a unit test in the module's `tests.rs` for the logic,
  and a case in `tests/` when the bug showed in real geometry (a joint turning the wrong
  way, a cabinet placed wrong). No fix is done before its test exists.
- **CI must stay green.** `.github/workflows/build.yml` runs `cargo fmt --check`, clippy with
  warnings as errors, and `python build.py` (unit and end-to-end tests against a downloaded
  SolidCraft CLI) on every push and pull request. Run `cargo fmt` before committing.
- **The sample kitchen shows everything.** A new function goes into `examples/kitchen.py` in
  the same branch; rerun it (it rewrites `examples/kitchen.sc` and the cut list CSV), and
  redraw the banner with `python docs/banner.py` when the cabinets look different.
- **Releases come from tags.** Pushing a tag `vX.Y.Z` that matches the version in
  `plugin.json` makes the same workflow attach `woodwork-X.Y.Z.zip` to a GitHub Release.
- **Release candidates often, full releases rarely.** New work goes out as `X.Y.Z-rc.N`
  (that version in `plugin.json` and `Cargo.toml`, tag `vX.Y.Z-rc.N`): a tag with a hyphen
  becomes a GitHub pre-release, which the README's latest-release link skips. When the
  candidates have settled, the version loses its suffix and `vX.Y.Z` is the full release.
- **Branches and pull requests, never straight to main.** One branch per function, one commit
  per function, with `docs/usage.md` updated (and `README.md`'s short list when a function is
  added) and `PLAN.md` ticked in the same commit. Do not push
  until the user says so.
- **Every length sent to the app carries its unit** (`"582 mm"`). A bare number in a size field
  is read in the design's unit, so an inch design would get 582 inches. Exceptions that take
  plain mm numbers only: `occurrence.move` `translate` and a joint's `point`.
- **Cabinets are parametric.** Sizes become design parameters named `<Prefix>_width` and so on
  (`kit::prefix` makes the name an identifier). Every panel and hole is an expression over
  them. The numeric values in the builders are for validating input only. Counts (shelves,
  dividers, doors, drawers) are fixed when the cabinet is built.
- **A cabinet is a component**, created inside whichever component is active, and the active
  component is restored afterwards. **A part that moves is a sub-component with a joint**
  (doors: revolute, drawers: slider). New moving parts follow the same rule. A part can hang
  on another moving part: a corner cabinet's second door is jointed to the first (`Door return
  fold`, or `Door fold` when the pair hangs on the return), not to the cabinet, so it follows it.
  The fold turns out of the cabinet, onto the first door's face.
- **A hinge follows the parameters.** `Kit::hinge` makes the hinge line twice, as a
  construction axis (`construct.axis.two_planes`): once in the door's component and once in
  what it hangs on, and `joint.create` mates the two axes. Both follow a resize, which a
  joint at a point (`joint.as_built`) did not: the door then swung about where the hinge
  used to be. The two planes are given outright in the command (`{plane: {origin, normal}}`
  with expressions), not made as features: a plugin cannot hide what it makes
  (`browser.visibility` in a plugin run is not kept), and plane features showed across every
  door. Such a plane is read in world coordinates when made and in the component's own
  afterwards, so a turned part (the diagonal door) gets its line before it is turned
  (`Kit::hinge_line`) and its turn as the joint's `angle`. The order of the two planes is
  the way the axis points, so the way the door opens. The axes sit in the cabinet's history
  folder. Four commands a hinge more than before. `solidcraft-cli snapshot` does not draw
  construction planes or axes, so a render does not show whether a design is cluttered.
- **A placed cabinet is held to its neighbour**, by a joint `<Name> link` from the cabinet
  it was placed against (side a) to the new one (side b), made by `link` in `lib.rs`.
  Beside it, the joint mates two **seats**: every cabinet has an upright construction axis
  at each back corner (`Kit::seat`, `<Name> seat L` and `seat R`; a corner cabinet has one
  at the end of its back leg and one at the end of its return, `seat L return`, where the
  wall is beside it and the placed cabinet stands a quarter turned, the joint's `angle`).
  The joint is a revolute between the two axes with limits 0 to 0: the app has no rigid
  joint between axes. Both axes follow their cabinet's parameters, so the two corners at
  the wall stay together whichever cabinet changes width, depth or return: sides against
  each other and backs flush. A seat is made while the cabinet is built, as a hinge line is,
  before it is moved. On top, or against a cabinet made before 0.1.8 (no seats), the joint
  is the older rigid one with the distance between the middles as its `offset`, an
  expression such as `A_height` (`select::Cabinet::toward`); that follows one way only.
  Both sides are then frames in world coordinates with `flip: true`. A held cabinet is
  not grounded: it is fixed through the chain to the first cabinet of the run, which is.
  The link is noted on the component (`link: {to, place}` beside the remembered values).
  Edit deletes the component and its joints with it, so it makes the cabinet's own link
  again and those of the cabinets that hang on it (`edit::links`, from `joint.list`).
  A cabinet with any link cannot be moved (`occurrence.move` answers `reverted: true`, and
  nothing fails), so Joinery suppresses all its links while the cabinet is on the origin.
- **A cabinet remembers how it was made.** `make` in `lib.rs` stores the entry and the dialog
  values on the cabinet's component (`edit::remember`, metadata key
  `io.github.tomasdjo.woodwork`; Joinery adds its kind there). Edit reads them, takes the
  sizes from the design's parameters instead, applies the filled-in fields, deletes the
  component and builds it again: same name, same parameters (`Kit` gets them as `_existing`
  and only changes one whose value the edit changed, so links survive), same placement
  (`component.list`'s transform, transposed for `occurrence.move`'s `to_matrix`), the boards'
  own materials, and the joinery drilled again. A new dialog field must survive this round
  trip; a field Edit should offer goes into `edit::merge` and the `edit` schema, where an
  empty number or `keep` means unchanged.
- **A hole is a cylinder cut, never `solid.hole`.** `fittings::hole`, the pin rows and Joinery
  cut `solid.cylinder` with `operation: cut` out of the one board named in `targets`. A
  `solid.hole` keeps the face it was picked on, and in a design with many cabinets that
  face is not found again when the history is rebuilt (`the reference ... no longer exists
  as it was picked`): deleting any cabinet in the sample kitchen failed the dowel holes of
  two wall units, and Edit deletes a cabinet. It only showed in the whole kitchen, so
  `tests/test_kitchen.py` builds it and deletes a cabinet.
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
- `select` and cabinet placement (`place`) read world bounding boxes, so they are wrong for
  a cabinet that is rotated off the axes.
- The cut list measures a board by its world bounding box when the board fills that box, and
  otherwise (an open door, a turned cabinet) by `model.mesh` in the body's own component: one
  extra command per such board, so about 250 of them at most. A board turned inside its own
  component (a sketch drawn at an angle) is measured by its smallest enclosing box, its
  thickness across its largest flat faces (`cutlist::extents`).
- A cabinet with doors or drawers cannot be made inside a component that has been moved:
  the hinge cup holes fail with "not on a face of a body" (the shift described above).
- A drawer's slide is `joint.as_built` at a fixed point from creation time. That is harmless
  for a slide (only its direction matters), but it is off if the cabinet is made inside a
  component that has already been moved.
- A cabinet is grounded at the end of a run, because a joint only moves against something
  fixed. Moving it later means ungrounding it first. A cabinet placed against another is
  held by its link instead; moving it means deleting that joint (and grounding it, or its
  doors do not open). Deleting the first cabinet of a run leaves the rest floating.
- A cabinet on top keeps its place sideways when the one under it changes width or depth:
  that link follows upwards only. So does any link to a cabinet made before 0.1.8.
- The first cabinet of a run is grounded on its own middle and grows to both sides, also a
  corner cabinet: half of a longer return then goes to the back. Held to a neighbour it
  grows away from it.
- A corner cabinet for a left corner placed right of a cabinet (or the mirror image) has
  its return's end there, not a back corner: it gets the older link.
- The hardware list is counted from feature and body names (`hinge cup`, `handle`, `pins`,
  `joint`, `Drawer n Front`), so renaming those breaks it.
- The cutting layouts (`nesting::nest`) are greedy guillotine packing into the free pieces of
  all open sheets: eight tries (four orders, best area or best short side fit, each piece split
  along its shorter leftover), the fewest sheets wins, then the emptiest last sheet. Free
  pieces are never joined again, so a cutting optimiser may save a sheet on a big job (the
  sample kitchen's 18 mm melamine: 13 sheets for 10.4 sheets of area). A part bigger than the
  sheet is only reported (`oversize`), not split.
- A board is a worktop, bought to length and not laid out on sheets, when the Worktop function
  made it, whatever its name (`lib.rs` finds it by its length parameter's comment,
  `worktop::ALONG`, and passes `_worktops`), or when its name contains `Worktop` (hand-made
  boards in older designs) (`cutlist::cutlist`).
- Edge banding and grain in the cut list go by the part's name (`cutlist::role`), with no
  per-part override (that would need `storage@0.1`). A cabinet's width axis is across its
  `Side L` in the world, or X for a board measured in its own component, so a cabinet turned
  off a quarter turn is right only for its turned boards. An L-shaped or diagonal corner
  shelf gets one front edge marked, not its two (or slanted) front edges. A material that is
  not ours has no grain (`material::GRAINED`).
- A box cabinet's drawers go over its doors when it has both (not with dividers), above a
  fixed `<Name> Rail` behind the gap; the shelves and pins are in the doors' part below it.
  `drawer_height` (over doors each front, else the top one with the rest equal) is the
  parameter `<Prefix>_drawer_height` only while it is used; an edit that stops using it sets
  it to 0 (`Kit::has`), so the next Edit does not take the old value up again.
- The gap round the fronts is the field `reveal`, 0.5 to 6 mm, always in mm whatever `units`
  says (read with `num`, made with `Kit::parameter_as` as `2 mm`; `edit::merge` does not
  scale it). It is the parameter `<Prefix>_reveal` on a cabinet with doors or drawers (kept
  once made, so an edit that puts fronts back takes it up). `fittings::DOOR_GAP` is only its
  default.
- `fronts: inset` (box cabinet only; the corner refuses it in `corner::input`, and its dialog
  has no such field): the fronts fill the opening between the panels, a reveal all round, the
  front face at y = 0 (the hinge line too, so the door's back edge swings away from the side).
  Shelves, dividers, pin rows and drawer boxes start at y = t, a panel shallower; sides, top,
  bottom and the Rail stay full depth, the Rail flush at the front between the drawers and the
  doors. With one door per compartment each door fills its own compartment. A drawer box's
  clearance is `BOX_CLEAR` alone when inset.
- A diagonal corner cabinet's front is always at 45 degrees: its door and plinth are components
  turned by a fixed angle (`Kit::turn`), since an occurrence's turn cannot follow a parameter.
  The depth at the return's end follows instead (`width - return + depth`). Their boards are
  cut with `solid.chamfer`, picking the edge by a point in mm that is worked out twice: as an
  expression for the board and as a number for the pick.
- A corner cabinet's `corner` field is auto by default (`corner::hand`): right when it is
  placed right of the selected cabinet, else left. `lib.rs` settles it before building, so
  the cabinet remembers left or right and Edit keeps it.
- A corner cabinet's two end panels are named `Side L` and `Side R`, so selection and placing
  see its whole footprint. Joinery refuses it (it has a `<Prefix>_return` parameter). Place
  puts a new cabinet against the selected one's Side R (right) or Side L (left), whichever
  way that panel faces (`select::Cabinet::seat`, read from the boxes of the two sides and the
  back): so one continues a corner cabinet's return turned a quarter, and a turned run carries
  on turned, corner cabinets included. Quarter turns only.
- Match selected links a new cabinet's parameters to the selected cabinet's (`B_height =
  A_height`; `kit::matched` passes them to `Kit` as `_match`). Deleting the first cabinet's
  parameters breaks the ones linked to it, and a matched cabinet gets a plinth only if the
  selected one has one. Material is not matched.
- Joinery covers the four carcass corners only, and tells an overlaid back from a grooved one
  by comparing the side depth with the depth parameter.
- Parts are modelled as an exact fit, with no clearance for the back panel or shelves.
- A board's label (`Kit::label`, the `labels` field) is its name only, not its size: text built
  from parameters takes whole parameters (`number(Width)`), not expressions. A board is told
  from its one size that is a thickness parameter, so the drawer boxes' 12 mm boards get
  none. The label sits in the middle of the board's bounding box, which on a corner
  cabinet's L-shaped board can be over the cut-out; a side's label is on its left face.
- Prices (`prices::estimate`): one price per material, for an 18 mm sheet of whatever sheet
  size the cut list lays out on (no price per sheet size), and other thicknesses scaled linearly
  (t/18). Banding and worktops are per metre, also in an inch design. The dialog cannot show
  the stored prices, a stored price cannot be removed (only set, 0 included), and its fields,
  none required, come in key order (`banding_*`, `currency`, `hardware_*`, `sheet_*`,
  `worktop`). A material that is not ours, or a price never set, is an unpriced line, out of
  the total. In an inch design the cut list adds `thickness_mm` to `sheets` and `length_mm`
  to `worktops` for this.
- DXF (`dxf::holes`): `model.faces` answers in world coordinates, also for a moved cabinet, a
  door in its sub-component and an open door, so a board's own axes are taken from its flat
  faces (the biggest is the thickness), not from the bounding box. Face areas are of the
  tessellation (a 5 mm hole's is 10 % short), so a hole's depth comes from its round face's
  centroid, half way down. Only holes along the thickness are drawn; holes into an edge are
  counted (`dxf_edge_holes`), grooves are not drawn. One `model.faces` per board with a round
  face (`face_types.cylinder` in `inspect.measure`), so past the command limit the rest get
  outlines only (`dxf_holes_skipped`). Holes are placed from the part's middle, so at Cut
  sizes they are up to half the banding off, and the part's length is told from its width by
  size, so a nearly square part's holes can be turned a quarter. Layer names write a decimal
  point as `P` (R12 takes no `.`).
- Place's Auto, the dialog's default, goes by the name of the selected board
  (`select::side`): `Side L`, `Side R` or `Top`, and the origin for anything else. A script
  that leaves `place` out gets the origin, as before.
- A worktop (`worktop`) is a grounded component with no joint to its cabinets (Edit deletes a
  cabinet's joints): its parameters are expressions over theirs (`<P>_width` the sum of the
  run's `_width`, or a corner cabinet's `_return` when that leg lies along the run, plus the
  end overhangs; `<P>_depth` the deepest cabinet's `_depth` plus the front overhang; its top on
  the anchor cabinet's `_height`), so it follows a resize and survives Edit cabinet. Its
  component stands on the run's fixed point (`worktop::build`, from `_held`, which `lib.rs`
  fills from `component.list` and the cabinets' `link` notes): a grounded cabinet's middle, or
  the side by which the run is held to a cabinet outside it; the board's corner is an
  expression from there. Grounding, ungrounding or linking a cabinet afterwards is not
  followed, nor a link of the older rigid kind (on top, pre-0.1.8), which holds a middle one
  way only. The deepest cabinet is chosen by its value when the worktop is made. Its body is named `<Name>` alone, and a name that a
  cabinet would claim as a part (`<Cabinet> ...`) is refused; a cabinet made later with such
  a name would still claim it. Quarter turns only, like Place. The run is read from the world
  boxes of each cabinet's sides and backs.

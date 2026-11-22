# solidcraft-woodwork

A plugin for SolidCraft (0.46+) that turns it into a woodworking tool: parametric cabinets,
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
  oneOf, bounds, lengths and `x-solidcraft-visible-when` (0.46; 0.45 cannot draw it). The dialog shows the fields that `required` names first, in that
  order, and the rest by key: so every schema lists all its fields in `required`, which also
  puts a ` *` after each label, except a field that may be left empty (Edit's numbers, the cut
  list's Save as), as a required field cannot be. `build.py` packs the manifest without white space: the app
  refuses one over 64 KiB. A choice is a `oneOf` list of `{const, title, description}` (0.43; `enum` shows the
  bare values, and 0.42 cannot draw `oneOf`). `tests/test_manifest.py` checks all of this.
  **Conditional fields (0.46).** `x-solidcraft-visible-when` hides a field that does not apply
  (`{field, eq}`, `{field, in}`, `all`, `any`, `not`; the controller is a sibling scalar field,
  the values valid for it, else the plugin is refused at load). A hidden field is left out of
  Run and the preview and is not required, so every reader takes its default when a key is
  absent, and a hidden field still has a default (it is in `required`, for the order). A hidden
  controller matches neither `eq` nor `in`, so `not` over a field that can hide is true: say
  "drawers 1 to 8" as an `in` list. Mutual conditions are a cycle and refused (drawers and
  doors cannot hide each other). No stored values. `x-solidcraft-defaults` gives each size an
  inch default when Units is Inches (`{when: {field: units, eq: in}, value}`), shown until the
  user changes the field; `build.plugin` applies it as the dialog does. `x-solidcraft-enabled-when`
  and `x-solidcraft-when` exist but are not used.
  A field's `description` (and an option's) is its hover tooltip: every field has one, one or two
  plain sentences for a hobbyist who does not know the trade words.
- **Tools for agents (0.45).** Every entry is also the tool `woodwork.<entry>` (`plugin.tools`,
  MCP `tools/list`), with its schema as the arguments. Over MCP the arguments are checked
  strictly first: a field `required` names must be there even when it has a default, and
  undeclared fields are refused. As nearly every field is required (for the dialog's order), an agent
  passes them all; the error names a missing one. The `woodwork.<entry>` command in a script
  (`approval` and `params`) and `plugin.run` pass the fields through unchecked, except for an
  entry with conditions (0.46): then they are checked as strictly (visible required fields
  present, nothing undeclared) and hidden keys are taken out. `build.plugin` therefore fills in
  the schema's defaults for the fields a test leaves out, as the dialog does.
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
- Limits per run: 256 app commands, 30 s (native command time included), 1 MiB result. The
  manifest asks for `runtime.permissions.clock` so that `clock.now` gives Unix seconds (0
  without it), which Drawing uses to stop in time (`lib.rs` `now`). No network access. Plugin
  data in the design (`storage@0.1`) holds only Woodwork settings; beyond that the design keeps
  component metadata and the drawing's fingerprints.
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
  misses on a component that has been moved (through the CLI directly it does not). A box made
  in a moved component lands shifted too, so Panel works on the cabinet at the origin (`inside`
  in `lib.rs`: it moves the cabinet there, square, and back). The builders make every cut while
  the new cabinet is still at its component's origin.
- A pattern of a hole can lose copies ("cut copies remove no material") when the component has
  been moved and another body stands where its local numbers point in the world. Patterns made at
  the origin (pin rows, joint rows) have not shown it, also after a cabinet in the sample kitchen
  is deleted.
- The plugin runs on a background copy of the design. From 0.45 the window applies the last
  view command of an accepted run (`view.fit`, `view.home`); the preview's dry runs leave the
  camera alone. `execute` ends a cabinet, corner or edit run with `select.clear` and
  `view.fit`: Fit frames the selection when there is one, such as the board a cabinet was placed
  against. `plugin.run` through the CLI reports it as `view: {kind: fit}`. Woodwork settings'
  `camera: leave` leaves `view.fit` out (`settings::fits`, one `storage.get` before
  `select.clear`); `select.clear` stays, so the next run does not place against a board picked
  before. The run takes one command more than before (three of the 26 `cabinet::build` leaves).
- `build.py` runs `solidcraft-cli plugin check` on the package: an error fails the build.
- Official docs: `docs/plugins.md`, `docs/plugin-runtime.md` and `docs/rfcs/0002-plugins.md` in
  `tangency-labs/solidcraft`. A `ui` capability is planned there.
- **Woodwork settings (0.46, `storage@0.1` and `dialog@0.1`).** `settings` writes its fields
  into our data in the design (`storage.patch {patch: {settings: ...}}`, lengths in mm,
  `data_format` 1; `settings::patch`). `cabinet` and `corner` have `dialog_hook`: the dialog
  calls `update-fields` (world `command-dialog-plugin` in `wit/plugin.wit`, added to our 0.1
  wit as upstream allows), `lib.rs`'s `dialog` reads `storage.get {plugin: "woodwork"}` (the
  hook runs outside a run, so it must name us) and `settings::fields` answers each stored value
  as the field's `default`, lengths in the dialog's `units`. The hook is a read entry on a copy
  of the design, 250 ms and 10M fuel: one command. `plugin.run` and the tools call it once
  before the entry, filling only keys left out; a hook entry's tool schema has no `required`
  (checked after the hook, with "is required" and no field name). The hook cannot write, so
  the last used values are not remembered; settings are per design, not per user. `camera`
  (`keep`, `fit`, `leave`) is stored like the choices but is no Cabinet field, so
  `settings::fields` skips it and the Settings dialog starts at Keep. `settings` has
  the hook too: its three numbers start at the stored value or the Cabinet default, so the app
  draws them as drag fields kept within their bounds, and `settings::patch` leaves out a number
  equal to the stored one (or that default), as the hook fills omitted keys for scripts too.
  `edit` has the hook to hide the other kind's fields (`edit::hidden`, `{visible: false}`,
  the lists `edit::BOX_ONLY` and `CORNER_ONLY` that `merge` checks too, to shrink as the
  kinds catch up; `wall_mount` is shown for a corner, as it is coming, but `merge` still
  refuses it there): its one command is
  `document.inspect {bodies: "names", timeline: false}`; the picked board (the `selection`
  value, else the design's selection) names the cabinet (`select::owner`), a corner one has
  `<Prefix>_return`. Nothing picked or no cabinet: no reply. It gives no `default` (in Edit an
  empty field is unchanged), and a failed read answers `{}`, as an error from the hook fails
  `plugin.run`. A field the hook hides is still sent (`sends_field`), so the run is the same.

## Layout

| File | What |
|---|---|
| `plugin.json` | Manifest: entries, dialog schemas, allowed commands, version |
| `src/lib.rs` | Entry dispatch (`execute`) and the wasm guest. The only place that runs commands |
| `src/<module>/mod.rs`, `tests.rs` | Each module below that has unit tests is a folder: the code in `mod.rs`, its tests in `tests.rs` beside it |
| `src/kit/` | Shared by the cabinet builders: size fields, design parameters, panels, moving parts on joints, history folder, summary |
| `src/fittings.rs` | Hardware both cabinet builders use: the constants, shelf pin rows, hinge cup heights, handle height and screw holes, legs and casters, and the wall mount (cleats, hanging rail, steel rail) |
| `src/hardware/` | The hinges' and drawer slides' shared sizes (`HW_cup_d`, `HW_slide_t`, ...): design parameters `lib.rs` makes the first time a cabinet's commands use them |
| `src/cabinet/` | Box cabinet builder: parameters in, list of commands out. `mod.rs` is the carcass and the order of things, `input.rs` reads and checks the fields, `doors.rs` and `drawers.rs` are the fronts |
| `src/corner/` | Corner cabinet builder, L-shaped or diagonal, written for a left corner and mirrored for a right one. `mod.rs` is the carcass, `input.rs` reads the dialog, `doors.rs` the L's bi-fold pair, `diagonal.rs` the diagonal front's door and plinth |
| `src/frame/` | Frame: a leg-and-rail base (legs, aprons, stretchers, dowel or bolt holes), built on `Kit` like a cabinet. `is_timber` tells its parts to the cut list |
| `src/edit/` | Edit cabinet: what a cabinet's component remembers (`component.metadata.set` under our key), and merging that with the Edit dialog and the design's parameters |
| `src/joint/` | Joinery chosen at build: a `Joint` (where one board's end meets another's face) and the cabinet's `Joinery` (its methods, their sizes as parameters, the cuts and the fastener counts) that the builders share |
| `src/panel/` | End panel or filler (with its return) on one end of a box cabinet: boards inside its component, sized by expressions over its parameters. `lib.rs` runs them in the cabinet's component (on the origin: `inside`), remembers them for Edit, and refuses Place against a side that has one |
| `src/worktop/` | Worktop over the selected cabinets: measures their feet (a corner cabinet's two legs), groups them into runs by the way they face, joins runs in corners (butt or mitre) and makes each piece a box or an extruded outline at world coordinates; dog holes and a cut-out in piece 1. `lib.rs` finds the cabinets (a picked worktop's remembered ones), replaces a worktop of that name, makes the component and grounds it |
| `src/material.rs` | Board materials: create on first use, assign |
| `src/cutlist/` | Cut list, edge banding and grain from `inspect.measure`, and the sheets to buy from the cutting layouts |
| `src/nesting/` | Cutting layouts: parts packed onto sheets per material and thickness, with kerf, trim and grain, in guillotine cuts a panel saw can make |
| `src/pdf/` | The cut list as an A4 PDF, written by hand with the viewer's own Helvetica (WinAnsi text, no embedded font), with a cutting diagram per sheet |
| `src/dxf/` | The cutting layouts as DXF R12, a file per sheet, written by hand: sheet, part outlines and names, and holes (read with `model.faces`, mapped onto the part as it lies on the sheet) on a layer per diameter and depth |
| `src/drawing/` | Shop drawings: a sheet per cabinet in the drawing `Woodwork`, as steps whose args name ids from earlier replies (`$sheet`, `$front`): scale, layout, the dimension corners and the board list as notes |
| `src/select/` | Finds the selected cabinet and its parts by body name |
| `src/settings/` | Woodwork settings: the storage patch from its dialog, and the Cabinet and Corner dialogs' defaults from what is stored |
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

Entries today: `cabinet`, `corner`, `frame`, `edit`, `material`, `panel`, `worktop`, `cutlist`, `drawing` and `settings`, all edit.

A new kind of cabinet is a new module that builds on `Kit` and a new entry with its own dialog.
`lib.rs` runs `cabinet`, `corner` and `frame` through the same steps (component, material,
place, ground), so a builder only returns its commands and `Kit`'s summary.

## Build, test, install

    python build.py            # cargo test, wasm build, package, end-to-end tests
    python build.py corner     # the same, but only the end-to-end tests named like that
    python build.py --install  # the same, then install into SolidCraft
    python examples/kitchen.py out.sc

- Needs Rust with the `wasm32-unknown-unknown` target and `wasm-tools` (both in `~/.cargo/bin`,
  which is not on PATH in a fresh shell; `build.py` adds it).
- The end-to-end tests in `tests/` run the built plugin through `solidcraft-cli` and check
  real geometry (bounding boxes, hole counts, joint moves). A new feature gets a case there.
- They run in parallel, `min(4, CPUs)` at a time (`WOODWORK_TEST_JOBS=1` for one by one), so
  a test saves under a file name of its own in `dist/`, and a profile test holds
  `helpers.own_profile`'s lock (its environment reaches only its own thread's runs).
- `solidcraft-cli snapshot --in file.sc --out shot.png` renders a result; look at it before
  calling a geometry change done.
- Output goes to `dist/` (ignored by git).

## Rules

- **Versions move slowly.** Bump only the last digit, in both `plugin.json` and `Cargo.toml`,
  unless the user decides on a bigger step (0.2.0). 0.1.0 was the first public release.
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
  nothing fails), so `inside` (Panel) suppresses all its links while the cabinet is on the origin.
- **A cabinet remembers how it was made.** `make` in `lib.rs` stores the entry and the dialog
  values on the cabinet's component (`edit::remember`, metadata key
  `io.github.tomasdjo.woodwork`; the Joinery function of 0.2.0 and before added `joinery`, its
  kind, there: `edit::merge` turns it into the same `joints` at build). Edit reads them, takes the
  sizes from the design's parameters instead, applies the filled-in fields, deletes the
  component and builds it again: same name, same parameters (`Kit` gets them as `_existing`
  and only changes one whose value the edit changed, so links survive), same placement
  (`component.list`'s transform, transposed for `occurrence.move`'s `to_matrix`), the boards'
  own materials, and the joinery made again. A new dialog field must survive this round
  trip; a field Edit should offer goes into `edit::merge` and the `edit` schema, where an
  empty number or `keep` means unchanged.
- **A hole is a cylinder cut, never `solid.hole`.** `fittings::hole`, the pin rows and the joints
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

- The dialog cannot remember the last values (only Woodwork settings' stored defaults). A size the user changed stays as typed when Units changes.
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
  `joint`, `Drawer n Front`, `Leg n`, `Caster n`), so renaming those breaks it. Joinery at build
  counts from parameters (below). For a design drilled by the removed Joinery function (0.2.0
  and before), the cut list still counts its screws and dowels: the round holes in `Bottom`,
  `Top`, `Rail front`, `Rail back` of a cabinet with a feature `<Name> joint Side L Bottom
  pilot` (confirmat) or `... dowel end` (dowels).
- The cutting layouts (`nesting::nest`) are greedy guillotine packing into the free pieces of
  all open sheets: eight tries (four orders, best area or best short side fit, each piece split
  along its shorter leftover), the fewest sheets wins, then the emptiest last sheet. Free
  pieces are never joined again, so a cutting optimiser may save a sheet on a big job (the
  sample kitchen's 18 mm melamine: 13 sheets for 10.4 sheets of area). A part bigger than the
  sheet is only reported (`oversize`), not split.
- A frame (`frame`) is recognised by its `<Name> Leg front left` body (`frame::MARK`):
  `select::owner` takes it as it takes a cabinet's `Side L`, so Edit (`select::piece`) and
  Material find it, and `select::cabinet` refuses it (Panel, Drawing, Place against it).
  Its legs are `Leg front left` and so on, never `Leg <n>` (`fittings::is_leg`, hardware). Its
  parts (`frame::is_timber`: `Leg <words>`, `Apron ...`, `Stretcher ...`) are the cut list's
  `timber`, bought to length, not on sheets; its joints are the shared joinery's (`Joinery::fixed`: dowels
  with `joint_edge` a quarter of the rail and the spacing its height, so a row of two, and bolts
  by `joint_by("bolts")`, `single`, the nut from the face opposite `Joint::out` (the rail's
  inner), `face_t` the leg), counted from `<Name>_dowels` and `_bolts` (a frame is found by
  `MARK` in `cutlist::hardware`); a frame made before keeps its `<Name> dowel <n>`, `bolt <n>`,
  `barrel nut <n>`, counted by name. A stretcher has the apron's section. With bolts, rails that
  meet at a middle leg are doweled. It has seats, so Place right or left of a cabinet links it;
  on Auto a cabinet's top gives the origin. Edit's frame fields are `stretchers`, `frame_legs`
  (the frame's `legs`) and `joint` (`edit::FRAME_ONLY`; the Edit hook hides them for a cabinet,
  and every cabinet field, `edit::CABINETS` and the kinds' lists, for a frame); its sizes come
  from Parameters (`edit::SIZES`). Dowels that would meet in a leg (a corner's two rails, or a
  middle leg's split rails: a leg under 50 mm with 6 legs) and bolts from both ends of a rail
  shorter than `2*BOLT_PAST` are refused. A frame made
  inside a moved component has the hole shift above.
- A board is a worktop, bought to length and not laid out on sheets, when its name contains
  `Worktop` (`cutlist::worktop`), at its world box (`cutlist::turned` leaves it out, so a
  mitred or cut-out piece is not measured by mesh). A worktop named otherwise is a board, and
  so is a name with ` Tool ` in it (the tool station's shelf and battens).
- A worktop (`worktop`) is numeric: measured from the cabinets' Side L, Side R, Back and Back
  return world boxes, made at world coordinates in a component in the active component, which
  must stand on the origin, unturned. A frame counts as a cabinet (`worktop::is_frame`,
  `frame_foot`): one foot over its legs' world boxes, facing from its front left to front right
  leg turned a quarter, top at the legs' top. It does not follow a cabinet; running Worktop again with
  its name (or a piece picked) deletes its component and makes it again over the cabinets it
  remembers (`cabinets` in its metadata, entry `worktop`), with its remembered values when no
  cabinet is picked, the dialog's when one is. A picked piece belongs to the longest worktop
  name it fits (`worktop::owners`). Quarter turns only (a side that does
  not fill 80 % of its box is refused). A corner cabinet's leg is as deep as its end panel is
  long. Runs are grouped by the way they face; two facing the same way must share a wall, and
  facing each other only with a run across (a U). In a butt corner the longer run passes through
  (equal: the first found). End panels and fillers are not part of the footprint: give an end
  overhang. Dog holes (one cylinder and a pattern) and the cut-out go into piece 1 only.
  Tool station (`tool: benchtop`, `worktop::under`): an opening in piece 1 and, in the one
  selected cabinet whose two sides (each square to the run) stand either side of it with its
  middle within their depth, `<Name> Tool shelf` (18 mm) on `Tool batten L`/`R` (18 x 40),
  numeric like the rest, Birch plywood (`lib.rs` assigns it after the worktop's material). The
  cabinet's `Top` or `Rail front`/`Rail back` crossing the tool's space get the opening too
  (`worktop::crossing`; `lib.rs` cuts them in the cabinet's component with `inside`, the box
  taken into its frame by `worktop::local`); any other body of the cabinet in that space, or in
  the shelf and battens' box, is refused, and so is an opening past the back's front face. That cut is not remembered: Edit cabinet drops it, and a tool moved on a run again
  leaves the old one. `piece_of` counts the three tool parts as the worktop's, so a run again
  replaces them and picking one makes the worktop again.
- Edge banding and grain in the cut list go by the part's name (`cutlist::role`), with no
  per-part override (that would need `storage@0.1`). A cabinet's width axis is across its
  `Side L` in the world, or X for a board measured in its own component, so a cabinet turned
  off a quarter turn is right only for its turned boards. An L-shaped or diagonal corner
  shelf gets one front edge marked, not its two (or slanted) front edges. A material that is
  not ours has no grain (`material::GRAINED`).
- The gap round the fronts is the field `reveal`, 0.5 to 6 mm, always in mm whatever `units`
  says (read with `num`, made with `Kit::parameter_as` as `2 mm`; `edit::merge` does not
  scale it). It is the parameter `<Prefix>_reveal` on a cabinet with doors or drawers (kept
  once made, so an edit that puts fronts back takes it up: `Kit::has`). `fittings::DOOR_GAP` is
  only its default.
- `fronts: inset` (box and corner cabinets; the corner's dialog shows it with doors, and
  `corner::input` takes it only with doors): the fronts fill the opening between the panels, a
  reveal all round, the front face at y = 0 (the hinge line too, so the door's back edge swings
  away from the side).
  Shelves, dividers, pin rows and drawer boxes start at y = t, a panel shallower; sides, top
  and bottom stay full depth. With one door per compartment each door fills its own
  compartment. A drawer box's clearance is `BOX_CLEAR` alone when inset (beside doors it is
  a panel more, as overlay, to clear the rail).
  Corner (`corner::doors`, `corner::diagonal`): the L's pair is the overlay pair a panel back
  (back faces at `depth - t` and `return - depth + t`, the fold at the carcass's inner front
  corner), the reveal in from the end panels; the diagonal door's front face is on the front's
  line and its ends stop `t/sqrt(2) + reveal` along it from the ends' outer corners, as far as
  the 45-degree end panels reach (their inner corner). Shelves only (not bottom and top) and
  the pin rows start a panel back: the L's notch from `depth - t`, the diagonal's chamfer
  `sqrt(2)*t` longer. Seats and links do not change.
- A box cabinet with drawers and doors (one or two: drawers take no dividers) puts the
  drawers at the top or the bottom (`drawers_at`, cabinet and Edit) and a fixed `<Name> Rail`
  between the sides, centred behind the reveal between drawers and doors (at y = t when
  inset, as the shelves); the shelves and pin rows are on the doors' side of it. The rail is
  joined by `joints_inner`. `drawer_height` (beside doors each front, 150 mm when 0; drawers
  only, the top one with the rest equal) is the parameter `<Prefix>_drawer_height` only while
  it is used; an edit that stops using it sets it to 0 (`Kit::has`), so the next Edit does not
  take the old value up again, and an Edit that takes the doors away sets it to 0 unless it
  gives one (`edit::merge`). Set to 0 in Parameters the fronts have no height and the rebuild
  fails.
- A diagonal corner cabinet's front is always at 45 degrees: its door and plinth are components
  turned by a fixed angle (`Kit::turn`), since an occurrence's turn cannot follow a parameter.
  The depth at the return's end follows instead (`width - return + depth`). Their boards are
  cut with `solid.chamfer`, picking the edge by a point in mm that is worked out twice: as an
  expression for the board and as a number for the pick.
- A corner cabinet's `boards: split` (L-shaped only; `corner::input` ignores it on the diagonal)
  makes its bottom, top and shelves `<Part>` (the back leg's, whole width) and `<Part> return`
  (in front of it), with two `<Part> return dowel <n>` cuts, each one cylinder through both boards
  (`targets` takes two). The cut list counts those features as 60 mm dowels and takes a
  cabinet's width axis along its `Back` (across `Side L` without one), as a left corner's Side L
  is its return's end. The backs and ends stay whole: a 1400 mm corner's back (1364 wide, grain
  up) is still oversize on a 1220 sheet in a grained material.
- A corner cabinet's `corner` field is auto by default (`corner::hand`): right when it is
  placed right of the selected cabinet, else left. `lib.rs` settles it before building, so
  the cabinet remembers left or right and Edit keeps it.
- A corner cabinet's two end panels are named `Side L` and `Side R`, so selection and placing
  see its whole footprint (it is told by its `<Prefix>_return` parameter). Place
  puts a new cabinet against the selected one's Side R (right) or Side L (left), whichever
  way that panel faces (`select::Cabinet::seat`, read from the boxes of the two sides and the
  back): so one continues a corner cabinet's return turned a quarter, and a turned run carries
  on turned, corner cabinets included. Quarter turns only.
- Match selected links a new cabinet's parameters to the selected cabinet's (`B_height =
  A_height`; `kit::matched` passes them to `Kit` as `_match`). Deleting the first cabinet's
  parameters breaks the ones linked to it, and a matched cabinet gets a plinth only if the
  selected one has one. Material is not matched.
- **Joinery at build** (`src/joint/`): the cabinet's `joints` field (carcass: sides to bottom and
  top, a sink base's rails one fastener each, `Joint::single`) is `none`, `dowels` (the dialog's
  default), `confirmat` or `screws` (4 x 50 mm, a thinner confirmat: `SCREW`), `cam` (thread hole in the face board, shank and cam housing in the end board from the face
  away from `Joint::out`; `<Prefix>_cam_d`, `_cam_bolt`), `pocket` (a 9.5 mm pocket and a pilot,
  cylinders on an axis at 15 degrees, a fixed angle: plugin runs take a non-axis `axis`; the
  pocket in the end board's `out` face, or the other with `pocket_face: inside`), `biscuit` (a box cut through both boards
  and a pattern, sizes from the `biscuit` choice, `SLOTS`, not parameters; a count parameter per
  size) or `dado` (box cabinets only: the end board
  is `Joinery::extension` (`<Prefix>_dado_depth`) longer at each end in the builder, and the
  joint is one box cut of its volume in the face board, through front to back; nothing counted;
  a corner cabinet refuses it, as its L-shaped boards, chamfer picked by point and backs would
  all have to grow); absent is `none`, so a cabinet made before rebuilds as it was.
  `joints_inner` (read only when the cabinet has dividers or the rail between drawers and
  doors, `INNER`) does the dividers' ends into the bottom and top (`Joint` along Z) and the
  rail into the sides, rows along the shelves' depth from `ein`; and without shelf pins (`input.fixed`)
  each shelf, per compartment, into the
  side or divider at each end. A shelf right of a divider is `Joint::between`: half a pitch
  along, one fewer, so the two shelves' holes in a divider do not meet; two dados opposite in a
  divider are refused past `(t - 3 mm)/2`. So a cabinet made with this branch's joinery and
  no pins gets joined shelves on its next Edit; one with no remembered method has `none` and
  stays as it was.
  `Joinery::new` reads it, checks the sizes and makes the parameters they need, once each
  (`<Prefix>_joint_spacing`, `_joint_edge`, `_dowel_d`, `_dowel_length`: dialog fields in the
  dialog's unit, `edit::SIZES`, so Edit takes them from Parameters and only changes the
  method). `Joinery::joint` gives a joint's cuts: one cylinder (a dowel's through both boards,
  `targets` two; confirmat a clearance in the face board and a pilot in the end) and a
  `solid.pattern.rectangular` along the row, count `max(2, ceil((run - 2*edge)/spacing) + 1)`,
  an expression, so the number follows a resize; joints that run the same way with the same
  count share one pattern (`Row`). They are cut in the builder's last commands, before the pin
  rows (a cut into a board full of pin holes is slow), with the cabinet on its component's
  origin, where a pattern keeps its copies. A 2D pattern (bottom and top rows in one) made new
  bodies of the copies that missed the targets: not used. Time goes by the hole, about 70 ms
  each on this machine (each copy is a boolean), more in a big design: the default spacing is
  250 mm. Measured in the sample kitchen: a box cabinet 3-7 s, the corner cabinets 7-8 s (about
  4 without joinery). `finish` adds
  the parameters the cut list counts from, `<Prefix>_dowels`, `_confirmats`, `_screws` (the rows'
  counts summed; `parameters.change` on an edit, `0` when no longer used: `joint::COUNTS` go
  into `_existing` only), and `joint::hardware` names the items from them (the dowel's size from
  its parameters), for a cabinet that is still there (its `Side L`). Features `<Name> joint
  <face> <end> <method>`, `... pilot`, `... row`, in the cabinet's folder. A dowel's depth in the face is `min(t - 5 mm, (L + 2 mm)/2)`.
  A corner cabinet (`corner::Corner::joints`, `joints` only) joins the backs' upright ends (into
  the back leg's end and the back return), the back return's front end (into the return's end)
  and the bottom and top into the four uprights; with a wall mount the backs stand on the bottom
  and under the top (joints along Z). Split, each piece is joined to what it meets and the two
  keep their fixed `return dowel` pair (an edge joint; the cut list's `Dowel, 8 x 60 mm`).
  A corner cabinet's shelves stay on pins (fixed only on a box cabinet).
- Parts are modelled as an exact fit, with no clearance for the back panel or shelves.
- A board's label (`Kit::label`, the `labels` field) is its name only, not its size: text built
  from parameters takes whole parameters (`number(Width)`), not expressions. A board is told
  from its one size that is a thickness parameter, so the drawer boxes' 12 mm boards get
  none. The label sits in the middle of the board's bounding box, which on a corner
  cabinet's L-shaped board can be over the cut-out; a side's label is on its left face.
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
  (`select::side`): `Side L`, `Side R` (or a panel on it) or `Top`, and the origin for anything else. From 0.46 a
  script must pass `place` (the entry has conditions, see Tools for agents).
- A box cabinet's `kind` (cabinet and Edit, remembered like any field): `standard` or `sink`
  (no oven housing, by decision). A **sink base** has no `Top` body but `<Name> Rail front` and `<Name> Rail back`
  (`RAIL_D` 100 mm deep, top face at `_height`, the back one ending where the shelves' depth
  ends), so `select::side` sends Place's Auto on a sink base's rail to the origin, and a script
  selects it by a side or a rail. A depth that leaves less than two rails' depth for them is
  refused. Its dialog hides shelves, pins and dividers; `cabinet::input` drops them
  for a sink base without an error (Edit to a sink base keeps the remembered ones, unused).
  It takes doors or drawers, not both (refused; the cabinet dialog hides Drawers when there
  are doors), and 2 drawers or more: the top front is `<Name> False front`, a fixed body in
  the cabinet's component with its handle hole and no box or slide, and the boxes below are
  `eshelf_d/2` deep, clear of the trap (no U-cut). An Edit to standard gives full drawers
  again; the panel counts its drawers as overlay fronts. The back groove is not
  cut into a top it does not have. Its joinery goes into the bottom and, one fastener each,
  into the rails (`Joint::single`).
- `cabinet::build` refuses a cabinet of more than 230 commands (the run itself takes the rest
  of the app's 256), counting the shared hardware parameters it may have to make: a wide tall
  cabinet with many doors, shelves, pins and labels can reach it.
- **Hinges and slides** (box cabinets, corner doors' hinges too; `src/hardware/`, holes in `cabinet::doors` and
  `cabinet::drawers`): every hole is a cylinder cut (`fittings::hole`) whose size and place
  are expressions over the shared `HW_` parameters, made once per design (`hardware::parameters`,
  run by `make` when the builder's commands mention `HW_`). A door's cups (`<Door> hinge cup n`,
  which the cut list counts) and the plate's two holes in the side it hangs on (`<Side> hinge
  plate n.k`, cut after `Kit::hinge` so the cabinet is active), `HW_setback` behind the front
  of the opening (`ein`, the shelf pins' line); a hinge edge on a divider gets no plate holes.
  A drawer box's sides (`<Drawer n Left> slide k`, cut before `Kit::close` while the drawer is
  active) and the carcass sides (`<Side> Drawer n slide k`, after), 6 pitches apart; the box is
  `2*HW_slide_t` narrower than the opening, and one shallower than 250 mm is refused
  (`input::MIN_SLIDE`, numbers only). The box cabinet's cups are moved to the pins' grid
  (`hardware::on_grid`, from `Cabinet::shelf_zone`'s base, the rail's top under doors over
  drawers): a plate's two holes are pin holes or clear of the rows, never half into one. The
  grid is `HW_pitch` and the pin rows' `PITCH`, which part if `HW_pitch` is changed. Corner
  doors use the same (`hardware::cups`, `cup_holes`, `plate_holes`, which the box cabinet's
  doors call too): the grid over the bottom panel, the first door's plate holes in the end it
  hangs on (`corner::doors`, after its hinge), none for the fold (it hangs on a door) nor the
  diagonal door (its ends stand at 45 degrees).
  Shared hardware *models* (#70: definitions under a `Hardware` component, an `occurrence.copy`
  per hinge or slide on an axis joint, one `solid.combine` cut per board) worked in a plugin
  run but are too slow on 0.46.1: each copy re-evaluates every combine in the design, and once
  a design holds such cuts every `component.create` re-evaluates the whole design (the sample
  kitchen's runs went from 1-4 s to 25-30 s and over the 30 s limit). `browser.visibility` is
  still not kept after a plugin run, and a definition made inside a cabinet's component is
  deleted with it (and every copy elsewhere).
- Drawing (`drawing::sheet`): the drawing commands (0.45) run inside a plugin run; while a
  drawing is open the model's commands wait, so `lib.rs` reads everything first, then opens or
  makes `Woodwork`, draws, deletes the replaced sheets and closes it. The board list is four notes
  (a note's `at` is the middle of its text, so a column's left edge comes from our Helvetica
  widths), not a native parts list, which lists components only. A dimension's corner must be a
  corner of one body (sides and back, with the cabinet's occurrence `path`), or the whole run
  fails. Layout: front and right views left of `RIGHT`, centred; the iso view at its own scale
  (`fit`, from `SCALES`, 1:4, 1:8, 1:15 and 1:25 included) under the board list's longest column
  and over the title block, so a list reaching the title block leaves it 1:50. A corner cabinet
  (`corner_views`) is drawn from the side of its open corner, the iso along that corner's
  diagonal. Corner cabinets get no dimensions; quarter turns only; A3 and mm only. 12 commands a
  sheet. **Runs:** scope all skips a cabinet whose sheet is up to date: each sheet's
  `drawing::fingerprint` (FNV-1a over its steps, component and occurrence ids in) is noted in
  the drawing's property `io.github.tomasdjo.woodwork` (`{sheet id: fingerprint}`, written once
  at the end, `drawing.properties.set`; it comes back in `drawing.open`'s reply), and a sheet
  of the cabinet's name with the same one is kept (`drawing::up_to_date`). Then it draws sheets
  while `drawing::fits`: under `STOP_S` (20 s, by `clock.now`) and the 256 commands, at least one
  a run; each redrawn cabinet's old sheet is deleted right after its new one, so a stopped run
  leaves one sheet a cabinet. The reply has `sheets` (drawn), `remaining` and `message`; the
  app's status line only says Done, so the message is also logged (`log`). `drawing::budget`
  refuses only when turned boards leave no room for one sheet. Measured on the sample kitchen
  here: `drawing.open` (it answers with `drawing.inspect`, every title block's text) of the
  existing drawing took 6.5 s with 11 sheets: a sheet whose title block has a cell reading the
  component's `material` or `mass` weighs it each time (`title_block_text::with_values`
  upstream), cold. So each sheet is added with `drawing::unweighed` (the first sheet's block
  without those two cells): 2.7 ms for all 11. A sheet drawn before keeps its block until it is
  drawn again (it has no fingerprint, so the next All does). `inspect.measure` 1 s, a sheet
  about 0.15 s; CI is about three times slower. Doors are closed by a `pose` of the
  cabinet's ` hinge`, ` fold` and ` slide` joints at 0. The drawing is set to mm (create and
  `drawing.set`), also in an inch design. A sheet points at the cabinet's component and bodies:
  after Edit (which builds a new component) its views are blank and its dimensions lost until it
  is drawn again, and a deleted cabinet's sheet stays. The board list is text, so it does not
  follow a resize either.
- A panel (`panel`: `<Cabinet> End panel L`, or `Filler R` and `Filler R return`) is a body
  inside its box cabinet's component, every size an expression over the cabinet's parameters,
  anchored on its side by `±<Prefix>_width/2`. Like a hole, a box made in a moved component
  lands shifted in a plugin run, so `lib.rs` makes it with the cabinet on the origin (`inside`). The cabinet remembers its panels (`panels` beside the remembered
  values, each `{kind, side, units, width, thickness}`, one a side), and Edit makes again
  those whose first board was still there, before the materials are given back: the user deletes the panel's history folder with its features. Its
  material is the Side L's. Overlay fronts (the panel comes forward to their faces: doors, or
  drawers on a standard cabinet, not inset) and the plinth come from the remembered values (a
  matched cabinet's plinth from its parameter), read when it is made: an Edit that changes
  them makes the panel again to fit. The Kind field hides Filler width unless it is a filler.
  A side with a cabinet against it (the cabinet's own `link` note while its link is there, or
  the note of a cabinet hanging on it) is refused, but a cabinet standing there without a link
  is not seen. Place on a side with a panel (or the panel picked, on Auto) puts the new cabinet
  beyond it, unlinked, since the link holds the sides' seats together (`Cabinet::with_panel`); a drawing's views take the panels in (`panel::is_panel`),
  and its dimensions go to an end panel's or a filler's outer corners (with inset fronts the
  side's corners are the panel's too), so the width is over all; a filler's two boards count as
  one box. Corner cabinets are refused. No plinth return (an end panel reaches the
  floor, also beside a cabinet on legs; beside one on casters it starts at the plinth), no label on a panel. The filler width and a given
  thickness are plain lengths, not parameters. `inside` puts a moved or turned cabinet at the
  origin, square (`to_matrix`), and back: in a plugin run a box in a turned component landed
  shifted (`tests/test_panel.py`). An Edit with panels runs about a dozen commands
  more each, so a cabinet near `cabinet::build`'s 230 can then fail the 256 limit (the run is
  discarded, nothing is lost).
- `plinth_type` (cabinet, corner and Edit; the dialogs show it when the plinth is not 0 or Match
  selected is on; `fittings::plinth_type`, `board` when absent, so older cabinets keep theirs):
  `board` is `<Name> Plinth` (the corner's two, or the diagonal's turned one), `plinth_setback`
  back from the front: the field (shown for board or notch, 50 mm when absent: `KICK_RECESS`)
  is the parameter `<Prefix>_plinth_setback` while a board or notch uses it, kept once made
  (`Kit::has`, as the reveal), in Edit and matched (a selected cabinet without it leaves the
  dialog's: `Kit::size`). `notch` cuts
  `<Name> <Side> notch` (`plinth_setback` deep, plinth high) out of each side's (a corner's two
  ends') front bottom corner, no board; the cut list finds the notched sides by those feature
  names (`cutlist::notched`, passed in as `_notched`) and gives them a line of their own with
  `note`, at their bounding box (layouts and DXF too: the notch is not drawn); the drawing's
  board list marks them `(notched)` (`lib.rs` puts the list in `measure` as `notched`). `legs` puts the
  sides (a corner's ends and backs) on the plinth and makes `<Name> Leg <n>` bodies
  (`fittings::leg`, 40 mm cylinders, not in `Kit`'s bodies, so no material): four, six over
  900 mm wide, fixed when built; a corner gets one under each footprint corner (six on the L,
  five diagonal). Legs are not boards (`fittings::is_leg`: the cut list skips them and counts
  them as hardware, and `cutlist::turned` leaves them out, so no `model.mesh` for one); `select::named` takes the floor from them, so a cabinet placed beside one
  on legs stands on the floor, and a drawing's views take them in (its height dimension is the
  carcass's). The drawing's dimensions are on the sides' top front corners and Side L's rear
  bottom one, as a notched side has no front bottom corner. One long plinth board across a run
  is not made (PLAN.md). `casters` (box and corner cabinets; `fittings::plinth_type` refuses a plinth over 0
  and under `CASTER_MIN` 50 mm, 0 is no plinth; `fittings::foot` places a leg or a caster) stands the sides on the plinth as legs
  do and makes `<Name> Caster <n>` (`fittings::caster`: an 80 mm plate box under the bottom, a 20 mm
  post and a wheel of `min(75 mm, plinth - 2 mm)` on the floor joined, one body, plinth high) at the legs' places by the same loop, but 50 mm in
  from the inside of the sides, so the plate's four 4 mm pilot holes (`<Name> Bottom caster <n>`,
  one cut and a 2 x 2 pattern, 60 mm apart, `min(10 mm, t/2 - 1 mm)` deep from below, clear of a
  back groove) land in the bottom panel. Five commands a caster. `fittings::is_foot` (leg or
  caster) is what the cut list, `cutlist::turned`, `select::named` and the drawing use; the
  hardware line is "Caster, locking". An end panel beside a cabinet on casters starts at the
  plinth (it rolls), read from the remembered `plinth_type`. A corner cabinet's casters stand
  at its legs' places, `t + 50 mm` in from the ends and backs; split, the two at the return's
  end screw into `Bottom return` (`<Name> Bottom return caster <n>`). The post is joined before
  the wheel, which on its own would not touch the plate (a timeline warning).
- `wall_mount` (box and corner cabinets and Edit; `fittings::wall_mount`, `mount_sizes`, `steel_rail`, `mount_board`, shared): `cleat` makes `<Name> Cleat` and
  `<Name> Wall cleat` (a body in the cabinet's component, so it moves and is rebuilt with it),
  bevelled by `solid.chamfer` on a box edge picked by index, sizes the parameters
  `<Prefix>_cleat_height` and `_cleat_thickness`; `rail` makes `<Name> Hanging rail` (panel
  thickness, 90 mm fixed). Either sits under the top, flush with the sides' rear, and the back
  groove moves forward by its thickness (an overlaid back is refused). `steel` is only hardware:
  the parameter `<Prefix>_wall_rail` (the width, 0 after an Edit takes it away) gives the cut
  list a rail bought to length and two hangers; hanger models come with #70. The cut list gives
  the cleats as square boards (the bevel is sawn after).
  A corner cabinet gets them along both walls (`Cleat`, `Cleat return`, and the same for the
  wall cleats and rails; the return's bevel edges are the box's +x or -x ones, 11 and 0 or 7
  and 2): the backs stand the mount's thickness off the walls between bottom and top (`ech`
  high), bottom and top reach the walls (`w0` 0), the shelves and pin rows stop at the backs.
  Steel gives it `_wall_rail` and `_wall_rail_return` (`edit::SIZES` has both); the cut list reads
  either.

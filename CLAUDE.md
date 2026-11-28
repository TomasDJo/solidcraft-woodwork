# solidcraft-woodwork

A plugin for SolidCraft (0.50+) that turns it into a woodworking tool: parametric cabinets,
materials, joinery and cut lists.

**Woodwork targets the latest SolidCraft release.** Raise the minimum version freely when a new
release helps; older SolidCraft users keep the older Woodwork release (the README's version
table says which).
`README.md` is the short user-facing description, `docs/usage.md` how to use each function,
`PLAN.md` the roadmap with what is done and open.

## How SolidCraft plugins work

- A plugin is a folder with `plugin.json` and one WebAssembly **component**. It exports
  `execute(entry, params_json)` and can only call the app's ordinary commands through
  `commands.run` (the same commands MCP and `solidcraft-cli` use). Interface: `wit/plugin.wit`,
  upstream's `craft:plugin@0.2.0` (as in SolidCraft 0.49.0) with our world `woodwork` at its end (commands, log,
  control, files, clock, storage; exports entry and dialog-provider). wit-bindgen 0.63 parses it.
- The manifest lists the entries, a JSON schema per entry, and every command id the plugin
  calls. A command that is not listed is refused. Entry names: lowercase letters, digits,
  hyphens (no underscores).
- The app draws each entry's dialog from its schema and shows the entry under the Plugins menu
  (a Woodwork submenu, and one button per entry in a Woodwork group on the Plugins ribbon tab;
  a `ribbon` slot named after a host tab such as Model would put it there instead).
  A schema keyword the dialog cannot draw or check (`pattern`, `readOnly`, ...) leaves the
  entry in the menu with Run turned off; ours use only type, title, description, default,
  oneOf, bounds, lengths, `x-solidcraft-visible-when` (0.46; 0.45 cannot draw it) and `x-solidcraft-help`
  (a field's "More about this field" link into `docs/usage.md`, 0.50; 0.49 turns Run off;
  per field only, about 105 bytes each, so only where a hobbyist needs the guide). The dialog shows the fields that `required` names first, in that
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
  user changes the field; `build.plugin` applies it as the dialog does. `x-solidcraft-when` exists but is not used.
  A field's `description` (and an option's) is its hover tooltip: every field has one, one or two
  plain sentences for a hobbyist who does not know the trade words.
  **A refused combination is said at once (0.5.0):** the Cabinet hook runs the build's checks of
  the fields (`cabinet::checked`, `corner::checked`: `Kit::new` and `input::read`, a few us; a
  whole build costs too much of the hook's fuel), on the dialog's values with the design-wide
  settings filled in as the run does (no design read), and puts the refusal as the hint of the
  field it is about (`settings::problem`, by the message's words, else Shape; `on_cabinet` for
  a setting's field, as a reply naming a field the schema lacks is ignored whole). Edit picked:
  on the cabinet's file (`shown` and the sizes, through `edit::merge` of the change), only when
  the cabinet as it is passes. Woodwork settings: `settings::trouble` (a `patch` refusal on its
  field, or a standard cabinet's check with the changed settings), on a field of that dialog
  only. The 0.49 hook has no way to turn Run off or mark a field invalid (only `visible`,
  `enabled`, `choices`, `default`, `value`, `hint`), and integers take no `choices`, so Doors
  cannot be limited. A hook `hint` is only the field's hover tooltip in 0.49 (`plugin_menu.rs`
  `apply_hook_reply`: `field.tip`, drawn by `with_row_label_tooltip`), so nothing showed until
  the mouse was on Doors. A Problem row (a greyed text field near the top showing the reason)
  was tried and removed for 0.5.0: an Edit into a refused combination did not show it in the
  app, and SolidCraft's own preview errors (tangency-labs/solidcraft#22297) are to say it
  instead. The hook does run on every change (120 ms after the last).
  **Fields that cannot combine are hidden, and not used:** a field that another one makes
  meaningless gets a condition, and the build takes a stale value of it (still sent by a
  script, or remembered by Edit) as absent instead of refusing. Cabinet: Dividers with
  drawers (`cabinet::input`: none beside drawers or on a sink base; drawers beside doors on a
  sink base dropped); Sizing with Match selected (`from_drawers`: outside sizing); Width and
  Depth with drawer sizing (they are worked out; `sizing: drawer` and the inside sizes are
  remembered, and `rebuild` runs `from_drawers` again, so Edit and Apply settings keep the
  boxes' inside, and Edit shows Size by and the inside sizes, not the outside). Height, Depth, Thickness and Plinth are
  greyed, not hidden, with Match selected (`x-solidcraft-enabled-when`, 0.49: shown, still
  sent; `Kit` takes the selected cabinet's), and the Cabinet hook fills them with the picked
  cabinet's values and the hint "Match selected takes it from <name>" (`settings::matched_sizes`,
  from the same `document.inspect` as `matching`). Edit, where Keep and differing values
  leave a field shown: Dividers with drawers, Shelves and Shelf pins on a drawers-only
  cabinet, Slide length without drawers, Fronts, Reveal and Handles without doors and drawers
  (all as `not` over the number, so an empty number shows them), Handle position with handles
  None or hidden. Settings: Handle position with handles None, Plinth type (and so its set-back)
  with a plinth height of 0. Not expressed: Settings' Back against a Cabinet's Wall mount (two
  dialogs), Edit's Wall mount against Back (an option, not a field), Picked cabinets with Apply
  No (a keyword on the reserved `selection` may turn Run off).
  **Settings' sections (#128 step 2):** an entry per group, `settings-general`, `-fronts`
  (what doors and drawer fronts share), `-doors` (hinge opening, double dividers),
  `-drawers`, `-plinth`, `-joinery`, `-cutlist`, `-display` (menu "<Group> settings", the
  settings icon), each with only its fields. Same `execute` branch and storage: a run sends only
  its fields, so `patch` leaves the rest as stored. The hook (`settings::fields`) answers only
  the entry's fields (`settings::SECTIONS`), as a reply naming a field the schema lacks is
  ignored whole; a refusal (`settings::trouble`) is hinted only when its field is in the entry.
  A schema condition cannot name another entry's field: Finger-joint the bottom too (Joinery) is
  hidden by the hook when the stored `plinth_type` is legs or casters.
  `units` is a setting, in General only: the other entries' sizes are in the stored unit (the
  hook scales by it and hints "In mm, as set in General"; `patch` takes it when the run gives no
  `units`). There is no bare `settings` entry (it showed as an empty button): scripts and agents
  run the section entries, and `build.plugin(approval, "settings", {...})` splits the fields
  into one run a section (`build.settings_steps`, General first with the units). `runtime.ribbon_groups` (SolidCraft 0.50, tangency-labs/solidcraft#20089)
  puts the section entries behind one "Woodwork settings" button in the Model tab's Plugins
  group; it affects only those buttons, not the Plugins menu or the slot tabs, and an entry a
  user pinned keeps its own button. `tests/test_manifest.py` checks the group lists exactly the
  section entries.
  **Field order:** the most used first, and a conditional field right after the field that
  controls it, directly below it (drawer height and Drawers at under Drawers, the door hinge and
  hinge angle under Doors, plinth type and set-back under Plinth, slide length under slide type, a joinery method's sizes after it,
  the filler width after the panel's kind, the cut list's diagrams after its format). A field
  that may be left empty cannot be required, so it comes last whatever it controls: Edit's
  numbers (shelves, doors, drawers...) and the cut list's Save as. Settings' numbers have
  defaults (and are required) so they keep their place; the hook starts them at the stored value.
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
  `camera: leave` leaves `view.fit` out (`settings::fits`, from the one `storage.get` that
  `execute` runs first for these entries, which also gives a new cabinet its design-wide
  joinery and hardware); `select.clear` stays, so the next run does not place against a board picked
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
  the hook too: its numbers start at the stored value or the Cabinet default, so the app
  draws them as drag fields kept within their bounds, and `settings::patch` leaves out a value
  equal to the stored one (or the default), as the hook fills omitted keys for scripts too.
  **Design-wide settings** (`settings::CHOICES` and `joint::SIZES`): `joints`, `joints_inner`,
  the joint sizes, `biscuit`, `pocket_face`, `hardware_models`, `hinge_angle`, `slide_type`
  are no Cabinet or Corner field. `lib.rs` fills a new cabinet's params with them
  (`settings::design`: stored, else the default, lengths in its `units`; only keys absent, as
  the unit tests give them; a corner skips `joints_inner` and `slide_type` and takes dowels for
  a stored dado), so `make` remembers them and Edit rebuilds from those, not from the settings.
  Over `plugin.run` and the tools they cannot be passed (undeclared, refused on an entry with
  conditions): a script runs the settings' section entries first. The Settings dialog's choices keep `keep` (the
  test helper sends schema defaults) but the hook answers the stored value or the default, so
  each size's `x-solidcraft-visible-when` on the method works; `build.plugin` sends Settings no
  number (the hook fills them). Slide length stays per cabinet: Auto at build (it goes by the
  depth); a cabinet keeps what it was built with until Rebuild cabinets. Tests: `designed` and
  `setting` in `tests/helpers.py` (settings results left out, so indices stay the cabinets').
  Since then also `back`, `back_thickness` (box only), `fronts`, `handle_position`, `plinth`
  (the height, 100 mm by default), `plinth_type`, `plinth_setback`, `labels` (stored `yes`/`no`,
  given as a bool), `cleat_height`, `cleat_thickness`, `drawer_thickness`, `drawer_bottom` (box only, parameters while the cabinet has boxes),
  `drawer_side_height`, `drawer_back_height` (#221: box only, 0 the full height and the sides', a
  parameter only when set, the board's height `min(<box>, <param>)`, the back's too (#242: lower or taller
  than the sides; its corners join them over `min(<side>, <param>)`, whole above) per drawer, so a taller value (dialog, Parameters, Edit) is capped, not refused, and
  a sink's false front sets no limit; `input` refuses only 1 to 49 mm; on finger joints too
  (#233): the back corners' fingers over the back's own height, their own pair of rows when it
  differs from the sides', `fingers 3` and `4`); the cut list's `banding`, `cut_sizes`
  (its field `size`), `sheet`, `kerf`, `trim` (mm in every unit, `Def::Mm`); a worktop's
  `worktop_material` and `worktop_thickness` (its `material` and `thickness`; a worktop made
  again keeps its remembered ones). `settings::table` is the one list: key, the entry's field,
  `Def` (choice, length, mm number with bounds) and `To` (who takes it). Cabinet and Corner keep
  `plinth` as `yes`/`no` (`design` makes it the settings' height or 0; Match selected still
  takes the selected cabinet's in `Kit`); Edit's `plinth` is `keep`/`yes`/`no`
  (`settings::edit_plinth` before `merge`: Yes keeps a plinth the cabinet has, else the settings'
  height and type). `execute` reads the settings for `cutlist` too. Over `plugin.run` a size that
  a choice shows must come with that choice (the app checks the controller first: "is
  required"). `build.plugin` turns a test's moved fields into a group (Settings, the entry,
  Settings back to the defaults) that `build.cli` runs as one step (`flat`), and gives a cabinet
  without a plinth Plinth No; it sends Settings only `units`, `material`, `handles`, `camera`
  (the hook fills the rest).
  Settings only stores (no `apply` or `selection` field: Apply settings builds cabinets again).
  **See-through boards** (`see_through`, in `CHOICES` but no cabinet's: `design` skips it): each
  Settings run gives our design materials (`material.list`'s `user: true`, of `BOARDS`) the
  stored opacity, `SEE_THROUGH` 0.35 or 1, by `material.edit` of each that differs. It takes
  `opacity` only with `color` or `appearance`, so the look is given again, and a library
  appearance's preset PBR is opaque only ("opaque PBR requires numeric opacity 1"): `pbr: null`
  while see-through, left out (preset back) when solid. `material::assign` reads the setting
  (`storage.get`, one command, only when it creates a material) and creates it see-through.
  Oak is built into the app (`material.edit` refuses built-ins), so it stays solid. Settings is
  an edit entry, so its preview shows the change.
  **Edit (one cabinet, its own fields).** Edit is Cabinet with `mode` `edit` ("Edit picked";
  `new` by default, never switched by what is picked, as a picked side means Match selected):
  `execute` and `dialog` turn it into the internal `edit` (no entry of its own any more;
  `build.entry_step` sends `edit` so). New-only fields (name, match, place, units, thickness,
  material, corner, boards, reveal) show with `mode eq new`, Edit-only ones (fronts,
  handle position, back, plinth type, labels) with `mode eq edit`; a box's Shape is greyed on Box
  by the hook (`enabled: false`), and a corner refuses Box. Mode `auto` (the default) is decided by
  `select::auto` in both the hook and `execute`: a door, front, shelf, divider or drawer part
  picked is Edit (the hook answers Edit's patch with `edit::NEW_ONLY` hidden), a carcass board
  (Side, Top, Bottom, Back) or nothing is New (`edit::EDIT_ONLY` hidden; hidden fields are still
  sent, so `execute` drops them); Mode's hint says which. The hook cannot set Mode itself (a
  reply that does is refused whole by 0.49). `build.entry_step` sends Cabinet `mode: new`. Shelf pin holes is Cabinet's tick box
  (old files' `yes` read as true). Edit changes exactly one cabinet (`selection`
  `maxItems` 1; more is refused) and has only what is per cabinet: sizes (width, height, depth,
  a corner's return), kind or corner shape, doors, drawers with their height and side, shelves,
  pins, dividers, hinge side, fronts, handles and position, back, plinth yes/no and type, wall
  mount, labels (`edit::CABINETS`, `BOX_ONLY`, `CORNER_ONLY`). What is design-wide (joinery,
  hardware, hinge opening, slide type, thickness, reveal, plinth height and set-back) is Woodwork
  settings' and Rebuild cabinets'; `merge` refuses it (`build.plugin` turns such an Edit field into
  Settings, Rebuild cabinets and the setting back). Every field is required with a default and
  the hook starts each at the cabinet's value, so a run left as shown builds nothing
  (`edit::changed`, numbers as numbers, a value left out as `edit::assumed`, what the builder
  takes; a build lists `changed`). The other kind's fields are hidden and left out of the run
  (`edit::taken`). A conditional field given to `plugin.run` needs its controllers given too
  (`door_hinge` with `doors`): the host checks it against the schema defaults, not the hook's.
  A read entry may not read `component.metadata` (an edit command), so `make` keeps each
  cabinet's `edit::copy` (`edit::shown`, `entry`, `units`, its boards' first words `parts`) in a
  file of ours, `edit/<Prefix>.json` (`edit::blob`, the `_put_blob` pseudo-command), not in our
  main document, which every hook parses with 10M fuel. The hook stays light: the pick from the
  field (else the design's selection, `_selection`: only `document.inspect`'s `selection` is
  parsed, as the reply has every parameter; `selection.get` is an edit command to a read
  entry), the cabinet from our files' names (`storage.get`; a board is a cabinet's only if its
  file's `parts` lists its first word, so "Cabinet 2" without a file never gets "Cabinet"'s),
  that file (`_get_blob`), and its sizes as they are now (`_params`: `kit::param_value` finds
  `<Prefix>_width`... in the reply's text, in mm, without parsing it), then `edit::fields`.
  The release profile is `opt-level = 3` (less fuel than `s`). A cabinet made before has no
  file: the hook only hides the other kind's fields (`edit::hidden`, one
  `document.inspect {bodies: "names"}`), and the run writes the file and takes a field at the
  dialog's default as unchanged (`edit::unshown`; a change to exactly the default is lost that once). A failed read answers `{}`.

## Layout

| File | What |
|---|---|
| `plugin.json` | Manifest: entries, dialog schemas, allowed commands, version |
| `src/lib.rs` | Entry dispatch (`execute`) and the wasm guest. The only place that runs commands |
| `src/<module>/mod.rs`, `tests.rs` | Each module below that has unit tests is a folder: the code in `mod.rs`, its tests in `tests.rs` beside it |
| `src/kit/` | Shared by the cabinet builders: size fields, design parameters, panels, moving parts on joints, history folder, summary |
| `src/fittings.rs` | Hardware both cabinet builders use: the constants, shelf pin rows, hinge cup heights, handle height and screw holes, legs and casters, and the wall mount (cleats, hanging rail, steel rail) |
| `src/hardware/` | The hinges' and drawer slides' shared sizes (`HW_cup_d`, `HW_slide_t`, ...): design parameters `lib.rs` makes the first time a cabinet's commands use them; and their models: definitions under `Woodwork hardware`, copies in the doors and drawers, hidden from every `inspect.measure` |
| `src/cabinet/` | Box cabinet builder: parameters in, list of commands out. `mod.rs` is the carcass and the order of things, `input.rs` reads and checks the fields, `doors.rs` and `drawers.rs` are the fronts |
| `src/corner/` | Corner cabinet builder, L-shaped or diagonal, written for a left corner and mirrored for a right one. `mod.rs` is the carcass, `input.rs` reads the dialog, `doors.rs` the L's bi-fold pair, `diagonal.rs` the diagonal front's door and plinth |
| `src/edit/` | Edit cabinet: what a cabinet's component remembers (`component.metadata.set` under our key), and merging that with the Edit dialog and the design's parameters |
| `src/joint/` | Joinery chosen at build: a `Joint` (where one board's end meets another's face) and the cabinet's `Joinery` (its methods, their sizes as parameters, the cuts, the fastener models and counts) that the builders share |
| `src/panel/` | End panel or filler (with its return) on one end of a box cabinet: boards inside its component, sized by expressions over its parameters. `lib.rs` runs them in the cabinet's component (on the origin: `inside`), remembers them for Edit, and refuses Place against a side that has one |
| `src/base/` | Mobile base (#101, entry `base`): `base::plans` (each picked cabinet's plan box from its sides and backs, and the lowest point of its bodies) and `base::build`: at world coordinates with a `Kit` (numeric, like the worktop), `<Name> Frame front/back/left/right` on edge round the joint footprint, `Frame cross n` at each joint along the long way (the whole way across) and across it (`base::pieces`: only under the cabinets with that edge, stopped at the boards across, so no boards overlap, #216), `Frame stiffener n` across it so no two boards across are more than `stiffener_spacing` (600 mm, 300 to 2000) apart (`base::stiffeners`, #229; the boards along and the blocks keep clear of them, and a spacing that would put one on a corner block is refused), `Block n` in the corners and pairs between, evenly spaced at most `LONG` 1500 mm apart (`base::casters`, #228: a block on a board across moves to the nearest place clear of all of them, and a pair more comes when a move would leave a gap over `LONG`) with `fittings::caster` under each; summary `lift` (caster + frame height). Timber frame: `frame_material: timber` takes the frame's thickness and height from `timber_thickness` and `timber_width` (48 x 98 by default, the timber on edge; `base::timber`), and `lib.rs` gives it `material::TIMBER` (solid whatever See-through boards says), which the cut list buys by length: each piece in `timber` (shaped as `worktops`, the PDF's *Timber, bought to length*) and the metres per section in `timber_metres` (with `thickness_mm`, `width_mm`) and the CSV, on no sheet; `cost::estimate` prices each section a running metre (`Timber 48x98`, or with spaces); a caster block (`base::is_block`, `<Name> Block <n>`) is a piece noted `caster block`, out of the metres and not priced.  A long job (`"job": true`, #239: one run is about 30 host calls a cabinet, so 8 in a row passed the 256 of an ordinary run; no Preview) with a `dialog_hook`: the run saves its values (no `selection`, no timber `material`) in `base::file` (`base/<prefix>.json`), and the hook, for a picked body that `base::owner` finds (exactly one of `build`'s part names after a base's prefix, so a cabinet `Bench Frame` is not base `Bench`'s; only a base with a file: one made before has none until a run, no migration), answers that file as defaults (`base::fields`, Name's hint "Editing <name>"); a run whose pick holds a board of the base of its Name adds the other picked cabinets to the remembered ones (no removing yet). `lib.rs` makes it in a component, grounds it, moves each grounded cabinet up by `lift - floor` (`move_grounded`; a held one follows its link) and remembers `cabinets` in the component's metadata (entry `base`); run again by name, it deletes and makes it again over the picked or remembered cabinets; connector bolts (#198): `base::bolts` finds each pair of touching `Side L`/`Side R` of the picked cabinets in the world and gives four `BOLT_D` 9 mm holes through both, which `lib.rs` cuts in each cabinet's own component (`inside`, `worktop::local`) as `<Side> connector bolt n to <other side>` (on the side first by name, which the cut list counts as `Connector bolt, M6`) and `<Side> connector hole n to <other side>`, n the place in the pair, so the names do not depend on the pick order; frame screws (#198): `base::screws` gives two `SCREW_D` 4.5 mm holes down through each cabinet's `Bottom` into each frame board it rests on (its underside on the board's top, at least 100 mm along it, on its middle line), from `inspect.measure` of the frame's bodies (the summary's) beside the cabinets', cut the same way as `<Bottom> frame screw n into <board> at x, y` (`Bottom return` too; the world place in the name, so a run again with other frame sizes deletes the stale ones by `timeline.delete` in the cabinet and cuts the new ones), counted as `Frame screw, 4 x 50 mm`; the lift is rounded to the micrometre, as a bottom with holes measures a hair off; every cut skips a name already in the timeline and is not remembered, so an Edit drops it until Mobile base runs again; held to the base (#215): `on_base` gives each cabinet with no link of its own a rigid `<Name> link` from the base's occurrence (frames at the cabinet's origin, offset 0), ungrounds it and notes `link: {to: <base>, place: base}`; a run again grounds the cabinets `edit::hanging` from the base before deleting it, raises them, and links them again; `rebuild` with that note and the base still there puts the cabinet back at its matrix (a line still sizes it, `place_along` is skipped) and calls `on_base`; Apply settings (`rebuild`'s `follow_line`) instead places a cabinet along its line (on the floor, no link) and reports `rebase`, and `apply` then runs `execute("base")` once per base under any cabinet it looked at (#198: also ones it skipped, so a frame follows a resize; Edit cannot afford a base run in its 256 commands and says in its reply `base` to run Mobile base or Apply settings when `base::resized`) with its remembered `params`, nothing picked (`select.clear` first), which raises and links them |
| `src/worktop/` | Worktop over the selected cabinets: measures their feet (a corner cabinet's two legs), groups them into runs by the way they face, joins runs in corners (butt or mitre) and makes each piece a box or an extruded outline at world coordinates; dog holes and a cut-out in piece 1. `lib.rs` finds the cabinets (a picked worktop's remembered ones), replaces a worktop of that name, makes the component and grounds it |
| `src/material.rs` | Board materials: create on first use, assign; See-through boards (`see_through`, `opacity`) |
| `src/cutlist/` | Cut list, edge banding and grain from `inspect.measure`, and the sheets to buy from the cutting layouts |
| `src/nesting/` | Cutting layouts: parts packed onto sheets per material and thickness, with kerf, trim and grain, in guillotine cuts a panel saw can make |
| `src/cost/` | Cost estimate (#56): Woodwork settings' `prices` (free text, `name = price; ...`, a `Def::Choice("")` setting the cut list takes) read by `cost::prices`, and `cost::estimate` over the cut list's `sheets` (by material and `thickness_mm`), `banding` (metres) and `hardware` (the longest entry an item starts with): `lines`, `total`, `missing` (`<name> = ?` to copy). `lib.rs` puts it in the reply as `cost` and, with prices given, adds `cost::csv` to the CSV; the PDF draws a Cost table when a line has a price. Worktops are not priced. `scripts/usage_pictures.py` draws its picture from the saved PDF (`draw_pdf`, PyMuPDF) |
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
| `docs/workshop_card.py`, `docs/workshop/` | The SolidCraft Workshop pictures (`solidcraft-workshop.json`'s images), 1200x900 on `banner.py`'s dark background, one list (`SHOTS`): `docs/workshop-card.png`, the card's (no text: the card prints the name), and the plugin page's gallery in `docs/workshop/` (the sample kitchen, a see-through cabinet with fastener and hardware models, a drawer's finger joints, a corner cabinet). `python docs/workshop_card.py [name...]`. Rerun it when the cabinets look different |
| `scripts/usage_pictures.py`, `docs/images/` | The usage guide's pictures: one list (`SHOTS`: name, caption, steps, camera), each a small design rendered with `solidcraft-cli snapshot` (one at a time, plugin.run on `dist/woodwork`, never installed) and saved as a 128-colour PNG. `tests/test_docs.py` checks each is drawn and shown in `docs/usage.md`. Rerun when the cabinets look different |
| `examples/kitchen.py` | Builds the sample kitchen, `examples/kitchen.sc` and its cut list CSV, through the CLI. Rerun it when the plugin's output changes |

The pattern: a module is plain Rust that turns input into `Vec<(command, json args)>`, so
`cargo test` covers it without SolidCraft. `lib.rs` runs the list. Anything that needs a reply
from the app first (selection, measurements, the active component) is fetched in `lib.rs` and
passed in.

Entries today: `cabinet`, `edit`, `material`, `panel`, `base`, `worktop`, `cutlist`, `drawing`, the eight `settings-<section>` entries and `rebuild`, all edit.
A selection field (an array) has no `default`: the app's dialog refuses one with any keyword but
type, title, description, `maxItems` and the conditions, and the menu then says "This entry needs to
be run from the agent or CLI" (0.49.0, `selection_field`). `test_manifest` checks it.
**Apply settings** (entry `rebuild`, `"job": true`, SolidCraft 0.48): a long job (10 min, about a million
host calls, no Preview, Cancel leaves the design as it was). `lib.rs` `apply(params, stored, all,
settings)`: the cabinets of the bodies in its `selection` field (Cabinets, any part of each), or every
cabinet's `Side L` when it is empty (not the design's selection, which tests and scripts leave
behind), by `select::cabinets_of` (cabinet name prefixes; a hinge or slide model's copy, which
measures hide, the first name of its `(A:1/...)` path); box and corner only. Its dialog hook
puts `select::applies_to` on the field as a hint, with the prefixes from our files' names
(`storage.get`, no design read for the 10M fuel), so the dialog names the same cabinets as the
run; a board of a cabinet made before it had a file (pre-0.5.0) by its name before its board
word (`select::BOARD_WORDS`: `Base 1 Door 2` is `Base 1`'s). Only a body with neither, such as
a hardware model's definition, is "not part of a cabinet". Each one's `edit::merge` with no changes (sizes from Parameters), then `settings::apply`
(what the settings give it now: the design-wide ones stored or default, the hook's four only
stored, the plinth's only on a cabinet with one, and only what its entry takes; `None` when it
has them all) or as made, then `rebuild` (Edit's path, shared: delete, `make`, panels,
materials, place, links), with a `_progress` pseudo-command per cabinet that the guest's `run`
turns into `control.progress`. The reply: `applied`, `changed` (each cabinet's fields that
changed), `skipped` (with `why`) and `message`. A cabinet without a plinth keeps its plinth type
and set-back too, so it is skipped when nothing else differs.
Apply settings never overwrites what was chosen for a cabinet. Design-wide (applied):
material, thickness, reveal, joinery and its sizes, hardware models, hinge angle, slide type,
fastener models, back/drawer/cleat thicknesses, plinth height and set-back (on a cabinet with a
plinth). Per cabinet with a Settings default (`settings::PER_CABINET`): plinth Yes/No (by its
height), plinth type, handles, handle position, fronts, back, labels: it follows the new setting
only when the cabinet's value equals the default it was made with, which it remembers as
`_defaults` in its params (`settings::defaults`, set when made, updated by Apply settings; a
cabinet made before 0.5.0 has none: the built-in defaults). A hand-set value equal to that
default counts as the default and follows (the ambiguous case). Per cabinet only (never a
setting, kept): sizes, shape/kind, doors, drawers, drawers at, shelves, shelf pins, dividers,
door hinge, wall mount.
**One Cabinet dialog for box and corner cabinets:** its `shape` (`box`, `L-shaped`,
`diagonal`) shows the box's fields (`kind`, `doors`, `drawers`, `dividers`, ... on `shape: box`;
a field over `doors` says `doors in [1..11]`, as `not` over a field that can hide is true) or
the corner's (`corner`, `return`, `corner_doors`, `boards`); the corner's sizes and name are
conditional defaults on `shape` (`x-solidcraft-defaults` takes any condition, first rule wins).
`entries` in `lib.rs` sends a corner shape to `corner::build` as the entry `corner`, its
`corner_doors` as `doors`, so the cabinet remembers `corner` and Edit, the Edit hook and every
`entry == "corner"` test work as before. There is no `corner` entry (`woodwork.corner` is gone);
`build.plugin` still takes `"corner"` in the tests and sends Cabinet with a shape.

A new kind of cabinet is a new module that builds on `Kit` and a new entry with its own dialog.
`lib.rs` runs `cabinet` and `corner` through the same steps (component, material,
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
- **No backwards compatibility yet:** older designs and scripts may need rebuilding; don't add
  migration code unless asked.
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
- **Release notes show the major changes in pictures.** The workflow's generated notes only list
  the pull requests, so after the release is made, edit its notes (`gh release edit vX.Y.Z
  --notes-file`): a short section per major change (a new function, a new option that changes
  what is built) with a picture, and a before/after pair where the change alters something that
  existed (the old and the new version of the same cabinet, same camera). Bug fixes and small
  changes are a line each, without pictures. Render the pictures with `solidcraft-cli snapshot`
  from small designs built through the CLI (as the tests do; for a before picture, build with the
  previous release's package), all the same size and camera style, and upload them as release
  assets so the notes can show them.
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
- Door clashes (`cutlist::clashes`, #140, the reply's `clashes` and `Clash` CSV lines): a box
  cabinet's edge door (the one nearest a side with `<Side> hinge plate` features) and any other
  cabinet's ` Door`, ` Front` or integrated ` Box front` (its drawer has no `Front`) body within
  60 mm beyond that edge (an integrated front starts a side and a slide's gap in), in the same front plane
  (30 mm) and overlapping in height, both shut (thin across the front, 40 mm): opened past 90
  degrees the door swings in front of it. From names and world boxes only; corner cabinets
  (a `Back return`) and fronts open in the design are skipped. The door's hinge line, its
  front edge on the hinge side, is already the effective pivot of a concealed hinge (the open
  door stands within the cabinet's width, thrown forward by its thickness), so no linkage is
  modelled.
- The hardware list is counted from feature and body names (`hinge cup`, `handle`, `pins`, `Left slide 1 (<length>)`,
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
  sheet is only reported (`oversize`), not split. A corner's one-piece L board (`cutlist::l_boards`: its `<Board> notch` cut) is laid out
  as its square; `lib.rs` reads its open corner from the `model.mesh` it already takes (it fills
  no box: `turned`), `cutlist::open_corner` (the box corner no point reaches), passed as
  `_corners`, and each laid part carries `corner` (along x and y, at its far corner), which
  `dxf::part_outline` draws as an L in the DXF and the PDF's diagrams, the label on the bigger leg.
- The Frame function (a leg-and-rail base, entry `frame`) was removed: not useful on its own, a
  different design is planned in #101. With it went `src/frame/`, Edit's frame fields, the cut
  list's `timber`, the worktop over a frame, and the joinery's frame-only bolts and barrel nuts
  (`Joinery::fixed`, `face_t`). A design with a frame still opens; the plugin no longer knows it.
- A board is a worktop, bought to length and not laid out on sheets, when its name contains
  `Worktop` (`cutlist::worktop`), at its world box (`cutlist::turned` leaves it out, so a
  mitred or cut-out piece is not measured by mesh). A worktop named otherwise is a board, and
  so is a name with ` Tool ` in it (the tool station's shelf and battens).
- A worktop (`worktop`) is numeric: measured from the cabinets' Side L, Side R, Back and Back
  return world boxes, made at world coordinates in a component in the active component, which
  must stand on the origin, unturned. It does not follow a cabinet; running Worktop again with
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
  the shelf and battens' box, is refused, and so is an opening past the back's front face. Bolt holes
  for the tool's feet (#172): `bolt_x`, `bolt_y`, a rectangle of `BOLT_D` 8 mm cylinder cuts through
  the shelf round the tool's middle, each checked to be on the shelf; summary `tool.bolt_holes`. Side wing (#172):
  `wing` (none, left, right) and `wing_length`, `<Name> Wing` a box past piece 1's end (square
  ends only: a mitred end is refused), as deep and thick, on `<Name> Wing Leg 1`/`2` (`LEG` 40 mm
  square, `LEG_IN` 30 in from the free end and the edges) from the lowest point of the picked
  cabinets' bodies to the top's underside; `fittings::is_leg` takes them (hardware, not boards). That cut is not remembered: Edit cabinet drops it, and a tool moved on a run again
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
- Match selected (`match`) is set by the Cabinet hook: `settings::matching` answers
  `{default: selected, visible: true}` when the picked board (the field, else the design's
  selection) is a box or corner cabinet's (`select::owner` and its `Side L`),
  else `{visible: false}`. That is the hook's second read, `document.inspect {bodies: "names"}`
  after `storage.get`: 1-3 ms on the sample kitchen, well within its 250 ms. A hidden field is
  still sent, so the run uses `match` only when something is picked or selected
  (`entries`: `matching && !picked.is_empty()`).
- Match selected links a new cabinet's parameters to its run's shared group (#137:
  `B_height = Run_A_height`; `kit::matched` passes `{prefix, values, names, setup}` to `Kit` as
  `_match`, `names` the group parameter per size). The first match against A makes the group
  (`parameters.add Run_A_<key>` with A's expression, or its value when the expression names one
  of A's own parameters, `names_own`, then `parameters.change A_<key> =
  Run_A_<key>`: `setup`, run by `entries` before the build); a cabinet whose parameter is just a
  `Run_` name already shares that run. So deleting any cabinet breaks no other, and changing one
  cabinet's own parameter (Parameters, Edit, or Apply settings changing its thickness or plinth)
  takes only that size out of the run. An older `_match` without `names` points at
  `<prefix>_<key>` as before; no migration. A matched cabinet gets a plinth only if the selected
  one has one. Material is not matched.
- **Joinery at build** (`src/joint/`): the cabinet's `joints` (a Woodwork setting, filled in by
  `settings::design`; carcass: sides to bottom and top, a sink base's rails one fastener each,
  `Joint::single`) is `none`, `dowels` (the default), `confirmat` or `screws` (4 x 50 mm, a thinner confirmat: `SCREW`), `cam` (thread hole in the face board, cam housing and shank in the end board from the face
  away from `Joint::out`, the housing cut first: the shank into it is about a third cheaper than the other way round, #173; `<Prefix>_cam_d`, `_cam_bolt`), `pocket` (a 9.5 mm pocket and a pilot,
  cylinders on an axis at 15 degrees, a fixed angle: plugin runs take a non-axis `axis`; the
  pocket in the end board's `out` face, or the other with `pocket_face: inside`), `biscuit` (a box cut through both boards
  and a pattern, sizes from the `biscuit` choice, `SLOTS`, not parameters; a count parameter per
  size) or `dado` (box cabinets only: the end board
  is `Joinery::extension` (`<Prefix>_dado_depth`) longer at each end in the builder, and the
  joint is one box cut of its volume in the face board, through front to back; nothing counted;
  a corner cabinet refuses it, as its L-shaped boards, chamfer picked by point and backs would
  all have to grow), or `finger` (#131: box cabinets, carcass only; `joints_inner` refuses it):
  `Joinery::extension` is the thickness, so the end board runs through the face board, and each
  joint is two box cuts in the overlap (`joint::finger`): `<face> finger <end>` from the second
  finger on, `<end> finger <face>` from the first, `w = run / (2*floor(run/(2*fw)) + 1)` with
  `fw` the parameter `<Prefix>_finger_width` (auto, 0: the expression `<Prefix>_thickness`),
  repeated every `2w` by two shared rows (`<Name> fingers <n>`) for the whole shell: 8 cuts and
  2 patterns a cabinet. The cut list notes the boards (`cutlist::fingered`, `_fingered`).
  Over a plinth board or notch (`closed_bottom` in `Joinery::new`) the bottom's joints take the
  third scope, `BOTTOM`: `none`, butted, unless `finger_bottom` is yes
  (Woodwork settings, design-wide), as its fingers would be closed mortises. Corner cabinets
  (`corner::Corner::fingered`, `corner`): every corner joint, the boards grown by `t` on their
  fingered edges in `carcass` and a `t x t` square cut out of the bottom and top where two
  uprights meet (`<Part> corner back left`, ...); butted (`joint_by("none")`) where it is a
  T joint (a wall mount's backs on the bottom, under the top, against the ends' middles) or
  would move the diagonal's chamfer (bottom and top to the ends); no other board has them (the top
  and a sink base's rails are flush with the sides' tops).
  Drawer boxes: `drawer_joints` (none / finger; Woodwork settings, box only), read as `Input::box_fingers`; `cabinet::drawers` runs the box's front and back
  through the sides and cuts the four upright corners while the drawer is open (in its
  component), fingers as wide as the box boards (`drawer_thickness`), 8 cuts and 2 patterns a
  drawer. The cut list notes the boards (`cutlist::fingered`, `_fingered`);
  absent is `none`, so a cabinet made before rebuilds as it was.
  `drawer_front` (#163; separate / integrated; Woodwork settings, box only, design-wide like
  `drawer_joints`), read as `Input::integrated`: integrated makes no `Drawer <n> Front`; the
  box's `Box front` takes its height, depth place, thickness (`et`) and handle holes, and the
  box's x and width (#223: flush with the sides, so the fingers stay open; the slide gap
  shows), the sides, back
  and bottom stay at box size behind it (butted); with finger joints the sides run forward
  through it (`side_y` = `efront`, `box_d + et` deep) and the corner cuts are `et` deep. The cut
  list bands it as a front when its drawer has no `Front` (`cutlist` before `role`). The hardware
  list's plain `Drawer slide, pair` (from `Front` bodies) counts no integrated drawer, which
  always has named slides. Absent is `separate`. A sink base's false front is unchanged.
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
  (`<Prefix>_joint_spacing`, `_joint_edge`, `_dowel_d`, `_dowel_length`: settings in mm, given
  to the builder in the cabinet's unit, `edit::SIZES`, so Edit takes them from Parameters and only changes the
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
- **Handle-less fronts** (`handles` `grip`, `groove`; #180, `fittings::grip`): no screw holes, so
  no handles in the hardware list. A groove is one cylinder cut along the edge (20 mm, its axis
  in the front face, 25 mm from the edge); a grip edge is a box made on the back edge, turned
  45 degrees about it (`solid.move`) and cut away by `solid.combine`, all by names, never a
  chamfer's edge pick (the naming depth, below): 3 commands a front. A drawer front's top edge,
  or its bottom on Handle position Bottom (#230); a door's top or bottom by `fittings::grip_on_top`
  (auto by floor or wall unit, also on a tall door; middle is the top for a groove, refused for a
  grip edge), on the corner's folding door only. A grip edge's front is `grip_gap` (Woodwork
  settings, design-wide, the parameter `<Prefix>_grip_gap`; `fittings::grip_gap` checks it and
  that `MIN_GRIPPED` of the lowest front is left) shorter at its grip edge (`fittings::gripped`,
  `Cabinet::gripped`), a drawer's box with it, so a finger fits behind the bevel. `fittings::profile_fits`
  refuses a front too thin for it. Both run a millimetre past the front's ends.
  Handles are two Woodwork settings, no Cabinet field: `handles` (Drawer handles, Drawers
  section, with `tab`) and `door_handles` (Door handles, Doors section, no `tab`; `none` by
  default; `fittings::door_handles`, `Input::door_handles`, a corner's doors' `handles`), both
  `To::Cabinets`, `PER_CABINET` and Edit-only fields (`edit::EDIT_ONLY`), like `fronts`.
  `Cabinet::handles` picks a front's by its part (`Door...`); the grip gap goes on the doors or
  the drawers by their own handles (`gripped_door`, `gripped`). Fronts' Handle position and
  Grip gap stay shared, without a condition (a condition cannot name another entry's field).
  `build.plugin` gives a test's `handles` (not `tab`) to the doors too, as one field did before.
  `tab` (#181, drawers only; a sink base's false front gets nothing): the drawer's
  `Bottom` runs forward under the front as the tab: `cabinet::drawers`' `tab`, after each
  branch's bottom, joins `Bottom tab 1/2` (boxes) and `corner 1/2` (cylinders, `TAB_R`) to it,
  `min(<Prefix>_tab_width, box inside)` wide (`tab_width`, Woodwork settings' Drawers, 120 mm,
  a parameter while drawers have the tab, `edit::SIZES`) and `TAB_OUT` (12 mm) past the front's face,
  with a `Bottom tab cove` cut along it from below (#256: `COVE_R` 6 mm, its axis
  `COVE_R - min(COVE_DEPTH, bottom/2)` under the underside, so it cuts `min(COVE_DEPTH,
  bottom/2)` deep, half `TAB_OUT` in front of the face);
  a `<front> tab notch` from the front's lower edge up to the bottom's top (`Kit::targets`),
  a `Box front tab slot` when the bottom stands behind a box front (not on runners, not
  integrated); no scoop (#244). The front keeps its height (the gaps
  stay); the bottom stays at its height, above the front's lower edge, as the lowest drawer's
  must clear the carcass. The cut list measures the bottom with its tab (mesh: it fills no box).
- **Grooved fronts** (`front_style` flat, vertical, horizontal: per cabinet with a Settings
  default, as `fronts`; `groove_shape` square or round, `groove_spacing`, `groove_depth`:
  design-wide, the parameters `<Prefix>_groove_spacing` and `_groove_depth` on a cabinet with
  fronts; #182, `fittings::grooves`, `front_grooves`): one cut (a box, or a cylinder centred on
  the face) the front's whole length and one pattern a front, `max(round(len/spacing), 2) - 1`
  grooves at `len/strips`, so they follow a resize. A V groove (`groove_shape` v) is a triangle
  sketched square to the groove at its start (`<cut> V`, its plane's origin and its points
  expressions with units over `_groove_depth`: a bare number in `sketch.line` is mm, text in the
  design's unit) and a `solid.extrude` cut along the front, which
  patterns; a turned box and its combine cannot be patterned ("cannot be patterned or mirrored
  yet"). 5 commands a front instead of 2.
  The depth stays clear of the cups (`t - CUP_DEPTH - 1`, or `t/2` without doors).
- **Shaker fronts** (`front_style` shaker, #183; `frame_width`, `panel_thickness`: design-wide,
  the parameters `<Prefix>_frame_width` and `_panel_thickness`; `fittings::frame`, `front`): the
  part's own name is a stile (a door's hinge stile, so cups, hinge models, `cut` and select work
  as before; a drawer's left one, so `Drawer n Front` still counts), plus `<part> stile`,
  `rail bottom`, `rail top` and `panel`, and one box cut of the panel's size through the four
  frame boards as the groove (`PANEL_GROOVE` 8 mm): 6 commands a front instead of 1.
  `Kit::pieces` notes the boards and `fittings::hole` cuts `Kit::targets`, all five, so cups and
  handle holes need no change (a cut into a target it misses is fine). A door's handle goes to
  the middle of the other stile (`handle_edge`). Drawer fronts lower than `framed` (2 frames +
  60 mm), and integrated ones, stay flat; such a door is refused. The cut list's `role` takes
  ` rail top`/` rail bottom` across, ` stile` and a front's ` panel` upright.
  `glass` (`Input::glass`): a framed front's panel (doors and drawer fronts) is `<part> glass`
  instead, `GLASS_T` 4 mm, not in `Kit::targets` (no hole into it). A glass drawer front's
  handle is at `frame_width/2` into its top rail on Handle position Top or Auto, the bottom rail
  on Bottom; middle is refused (`input`, with `Input::framed_drawers`, which
  `cabinet::drawers` uses too). The tab's notch cuts `Kit::targets` (the bottom rail). `fittings::is_glass` (a name
  ending ` glass`): `lib.rs` gives those bodies `material::GLASS` (always `GLASS_OPACITY`, not
  in `BOARDS`, so See-through boards leaves it) and the Material entry skips them; the cut
  list leaves them out of the boards and buys them as `Glass, 4 mm, L x W mm` by size.
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
- **Place Along a line** (#147 step 1, `src/line/`, `along_line` in `lib.rs`): the pick field
  `line` (`x-solidcraft-selection` accepting `sketch_curve`, `edge`, `face`; about 200 bytes with
  its condition), read before the build: `sketch.inspect` (the curve's ends by point id, the
  front toward the middle of the sketch's other points, else -Y or -X; its driving `length`
  constraint's `param`, matched by `l`, the curve's index), or `model.edges` and `model.faces`
  of the body (the edge's upright face by the edge's name `<face>|<face>`, its normal the front;
  a face's lowest edge). Level and along X or Y only (`line::level`). `line::place`: the origin
  half the box depth in front of the line, the left (`start_at`, seen from the front, along the
  front turned a quarter) side at `offset`, turned by the front (0, 90, 180, -90). Width from line
  sets `width` (and `sizing: outside`) and, over a driving dimension, `_line: {width: "<dim> -
  <offset>"}`, which `Kit::parameter_as` uses as the expression; an app-named `d<n>` is renamed
  `<Sketch>_<curve>` first (`parameters.rename`, `follow`). Step 2: `height_line` (a second pick,
  `sketch_curve` or `edge`, any direction: `line::sketch_length`, `edge_length`) sets `height` and
  `_line.height` the same way. `place_along` grounds the cabinet first, then turns it and moves it
  by `line::translate`: expressions over `Kit`'s `centre` (the summary's, `<P>_width` and
  `<P>_depth`, a corner's `<P>_return`), so the Start at side and the back stay on the line when
  the cabinet changes size (a grounded occurrence follows its translation's expressions only when
  grounded before the move: grounded after, it stays put). `edit::remember` keeps the line fields
  (not `_line`), and `rebuild` (Edit and Apply settings) reads the line again when they are there
  (`along`): the width and height (a value of a numeric parameter changed, a dimension's
  expression left), then `place_along` instead of the old matrix; a line that cannot be read
  leaves it where it stood, said in the summary's `line` (Apply settings' reply gathers them as
  `lines`). A command that fails ends the whole run, so `there` checks first that each pick's
  sketch (`document.inspect`'s `sketches`) or body (`body_names`) is still in the design. A
  cabinet placed otherwise drops the line fields (`LINE_FIELDS`), so it never moves onto one. Apply settings builds every cabinet
  along a line again (`changed: ["line"]`), as only the build reads it. A width or height typed in
  Edit turns that size's line off (`width_from_line: false`, `height_line: []`). Edges and faces
  are found again by index (ponytail: a cut into the wall can renumber them). Field order: Line
  right under Place, Height from line after Width from line. A selection sets it up (#212): the
  Cabinet hook's last block answers `place` and `line` defaults (and `height_line` `[]`, the one
  other field it shows without a default) when `line::from_selection` finds a sketch curve (from
  `document.inspect`'s selection, as `selection` takes no sketch curves; made
  `external_sketch_curve`) or an edge of a body no cabinet owns; answered on every call, as a
  reply that drops a value puts the field back, and a field the user edited keeps theirs.
  `plugin.run` drops a given field that Place Auto hides and does not restore it once the hook
  sets Place, so a script leaving Place out leaves those out too.
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
  It takes doors or drawers, not both (the cabinet dialog hides Drawers when there are doors,
  and `cabinet::input` drops drawers sent beside doors), and 2 drawers or more: the top front is `<Name> False front`, a fixed body in
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
  With three overlay doors or more (inset ones alternate as asked) the end doors hinge on the
  sides whatever `door_hinge` says
  (`cabinet::doors::hinged_left`), so only the doors between hang on a divider (the first of
  them on the side `door_hinge` says, the rest alternating); such an
  overlay door covers half of it and gets half overlay hinges (`hardware::HALF_OVERLAY`, the
  9.5 mm crank), its cups named `<Door> hinge cup n (half overlay)`, which the cut list counts
  as "Hinge, cup, half overlay" apart from "Hinge, cup" (#157). **Double dividers**
  (`double_dividers`, Woodwork settings, box only, `no` by default, given as a bool;
  `Input::double`, used only with overlay doors one per compartment): each divider is
  `<Name> Divider k L` and `R` side by side (`Cabinet::dividers`, `cell_x` with `2*t`), the
  shelves' pins blind from each board's outer face, the fixed shelves joined to their own
  board, and a door hinged there takes a straight hinge (crank 0) with its plate holes in that
  board (#176).
  A drawer box's sides (`<Drawer n Left> slide k`, cut before `Kit::close` while the drawer is
  active) and the carcass sides (`<Side> Drawer n slide k`, after), whole pitches apart as the slide's length allows; the box is
  `2*HW_slide_t` narrower than the opening, and one shallower than 250 mm is refused
  (`input::MIN_SLIDE`, numbers only). The box cabinet's cups are moved to the pins' grid
  (`hardware::on_grid`, from `Cabinet::shelf_zone`'s base, the rail's top under doors over
  drawers): a plate's two holes are pin holes or clear of the rows, never half into one. The
  grid is `HW_pitch` and the pin rows' `PITCH`, which part if `HW_pitch` is changed. Corner
  doors use the same (`hardware::cups`, `cup_holes`, `plate_holes`, which the box cabinet's
  doors call too): the grid over the bottom panel, the first door's plate holes in the end it
  hangs on (`corner::doors`, after its hinge), none for the fold (it hangs on a door) nor the
  diagonal door (its ends stand at 45 degrees).
  **Models** (`hardware_models`: `on` by default, `off`, `cut`; Woodwork settings and Edit, kept by
  `Kit::models`): `hardware::copy` pushes an `occurrence.copy` of `HW hinge` or `HW hinge crank
  <c>` (cup and arm, in the door, from `cup_holes`, so corner doors too, but not a corner's fold
  door, which has cups and no plate holes: `crank` None, a door-to-door hinge is not modelled; `hardware::hinge`:
  the caller gives the crank, 18 for inset, 9.5 for a box cabinet's hinge edge on a divider,
  else 0), `HW hinge plate` (in the cabinet, over each cup's plate holes, from `plate_holes`:
  none on a divider or the diagonal door) and `HW slide cabinet|middle|drawer <length>` (each
  drawer side; the cabinet member in the cabinet's component, the drawer member and, on a
  3-section slide, the middle member in the drawer's, in `cabinet::drawers`; the middle member
  moves the drawer's whole travel, as half would need a driven joint), with our own keys
  `_matrix` and `_cut`. A hinge is its cup, a `RIM` (40 x 1.5) on the door's back face and a
  `BOSS` over the cup where the arm starts, both outside the door so `cut` cuts only the cup; the
  plate a cross, its middle as high as the arm and `WINGS` (12 x 3) as high as the plate, over
  the screw holes (#140: the bounding boxes and the copies' places are as before). Sizes are constants after a Blum CLIP top 110° hinge on a 0 mm plate and a 45 mm ball-bearing slide
  (`PLATE_SIZE` 8.5 x 40 x 48, `ARM_SIZE` 11 x 52 x 16, `MEMBERS` 6/3/4 thick and 45/40/35
  high): the arm lies on the plate's outer face for the standard geometry (`ARM_REACH`, the cup
  6.5 mm inside the side: 22.5 in from the edge, 18 mm sides, 2 mm reveal; other thicknesses
  or reveals leave it off by the difference, as the definitions are shared), and a crank moves
  it that much towards the hinge edge. The
  turn is given by two unit axes: the model's -Y goes `into` the board and its -X `toward`
  (for a hinge the hinge edge from the cup, so the arm reaches that way: a right-hung door's
  hinge is turned half about Y; `upright` for the rest). `component` in `lib.rs` strips
  them, runs the copy and then `occurrence.move` with `matrix` (the translation column
  expressions, which a move keeps and follows; `occurrence.copy`'s `translate` takes numbers
  only): 2 commands a model. The definitions are components in `Woodwork hardware` at the root
  (`hardware::definitions` of `hardware::wanted`, the components a build copies: one slide
  definition a length, since a move cannot scale; made by `component` before the cabinet when
  one is missing, then back to the user's component): a definition inside a cabinet is deleted with it, with every
  copy elsewhere, and Edit deletes the cabinet. `browser.visibility` set in a plugin run is not kept
  (checked on 0.46.1 with `paths` on the component's placement: the same command through the
  CLI hides it, also after save and reopen), so `Woodwork hardware` is placed at `HIDDEN`
  (absolute `to_matrix`, half a turn about Z), its tops 1 mm under the floor, reaching
  forward from 15 mm in front of a 560 mm cabinet's back on the origin, under the first
  cabinet's back, away from the fronts: the iso and front views' Fit frame about 3 %
  wider than with it hidden (measured with `view.apply`); from below it shows. In a plugin run
  a body made in a moved component lands shifted by where the component stands, so a
  definition added later (a crank, another slide length) is made with `Woodwork hardware`
  back on the origin, and it goes to `HIDDEN` again after (before, such a definition's body stood
  at the origin, offset from its component by `HIDDEN`, so every copy of it floated that far
  (125 mm back, 30 up) from its cup or slide; in an older design it stays so until its
  component is deleted and the cabinets using it are edited; `tests/test_hardware.py` checks
  every hinge copy against its cup and every plate against its holes, Edit included). Every `inspect.measure` reply in a run goes
  through `hardware::hide` (`execute`), so the cut list, drawing, worktop, select, Edit's
  materials and panels never see a model; the hardware list still counts features (`hinge
  cup`, drawer fronts), so nothing is counted twice. A drawing's views are of the cabinet's
  component, so they draw the models in it. `cut` drops the cup cylinders and, after the
  build, combines each door with its copy (`hardware::world_name` from one `component.list`,
  the occurrences' names from the root), the feature named `<Door> hinge cup n`; outside the
  cabinet's history folder. On 0.46.1 every combine against a copy makes every later
  `occurrence.copy` and `component.create` re-evaluate it (tangency-labs/solidcraft#20787,
  for 0.47), so it is not the default; the app's version cannot be read in a run, so it is not
  refused there. `cabinet::build` counts a copy as 2 commands, a cut as 2 more plus one
  `component.list`, and the definitions it copies (`hardware::cost`); over 230 it builds the
  cabinet again with `off` and says so (`hardware_models` in the summary).
  **Slide length** (`slide_length`, no field: auto at build, a value an older cabinet remembers kept): `hardware::SLIDES`,
  250, 270, 300 to 650 mm and 10 to 28 in; `auto` is the longest of the cabinet's unit
  (`Kit::unit`) within the box depth less `SLIDE_CLEAR` (3 mm), and a chosen one that does not
  fit is refused (`hardware::slide_length`, read in `cabinet::input` as `Input::slide`, mm and
  the label `450 mm`). The label is the slide models' definition names, their bar length, and
  the back screw hole's place (`HW_setback + floor((L - 2*HW_setback) / HW_pitch) *
  HW_pitch`); the box side holes are named `<Drawer n Left> slide k (450 mm, full extension)`, and the
  cut list counts `Drawer slide, full extension, 450 mm, pair` from ` Left slide 1 (` (the
  rest of the drawer fronts, older drawers, as `Drawer slide, pair`). **Slide type**
  (`slide_type`, Woodwork settings and Edit, `BOX_ONLY`: `partial`, `full` by default, `over`;
  `hardware::SLIDE_TYPES`, `travel`: 75 %, 100 %, +25 mm) is the drawer joint's limit
  (`Input::slide_type`), and 2-section slides have no middle member. **Hinge opening**
  (`hinge_angle`, Woodwork settings and Edit: 95, 110 by default, 165; `Kit::hinge_angle`) is
  every door hinge joint's limit, the box cabinet's, the corner's first door and the diagonal
  door; a corner's fold stays 90 (it lays the second door flat on the first). The hinge model
  is the same for every angle. Hinges a door: `fittings::cup_heights`, 2 up to 900 mm, 3 to
  1600, 4 to 2000, 5 above (the first two cups at the ends keep their names). The cup's two
  screw holes (45 apart, 9.5 off its centre) are not cut: 2 commands a cup more.
  **Strip runners** (`strips`, #146; `Input::strip`, with `runner` for the play): the dado runners' drawer (bottom under a box `2*(strip_t + play)` narrower than the opening, no stop) on `<Side> strip <n>` boards (`<Prefix>_strip_thickness` 12, `_strip_height` 18: Woodwork settings' Drawers `strip_thickness`, `strip_height`, `Box`, shown with Slide type Strip runners; then parameters, `edit::SIZES`) under the bottom from `ein`, `eshelf_d` deep (the box's base raised by `max(0, strip_h - BOX_CLEAR)` and the box that much lower, so the strip clears the carcass bottom, rail or drawer below; play less than the strip's thickness), made with `panel` in the cabinet's component; no `runner_depth`. The cut list counts no slides for a drawer whose `Side L strip <n>` body is there; the strips' screw holes are not drilled.
  **Undermount** (`under_partial`, `under_full`; `Input::under`): box inside = opening - 42 mm
  (`21 mm - bt` a side, so `drawer_thickness` over 16 is refused), box as long as the slide
  (`hardware::UNDER`, `under_length` against the inside depth), bottom `UNDER_RECESS` 13 mm up,
  `<Box back> notch 1/2` (35 x 13), a 6 x 10 mm hook hole by each (the left one named
  `... Left slide 1 (<length>, <type>)`, which the hardware list counts), and `HW slide under
  <length>` (`UNDER_SIZE` 12 x 14) under each side in the drawer's component; no side holes.
  Sizes unconfirmed, from Blum's sheets. Sample kitchen, local: models on cost up to 1.5 s a cabinet run more than off.
  **Fastener models** (`fastener_models`, Woodwork settings, `off` by default by the user's
  decision: they are for looking, with See-through boards; remembered, absent is off; none
  with `hardware_models: off`; `Joinery::fasteners`): after a joint's
  hole `Joinery::fastener` pushes the same command without `operation` and `targets`, named and
  `body_name` `<cut> fastener` (a confirmat or screw: the clearance hole as long as the screw; a
  cam: a 6 mm rod from the bolt's tip, `cam_bolt` long, and the housing; a pocket screw: its
  pilot; a biscuit: its slot; a dowel: the hole), and the row adds a
  `solid.pattern.rectangular` of `bodies` (`<row> fasteners`, copies `<name> (2)`, a fraction of a
  millisecond; a pattern of new-body *features* names its copies `Body1`). One command a joint
  and one a row. `hardware::is_model` takes a name ending in ` fastener`, or ` fastener (n)`, so
  `hide` keeps them from every measure; the drawing's views show them, as the hinge models.
  **The naming depth:** SolidCraft names a feature's faces from its history only up to
  `MAX_DEPTH` 400 (`crates/doc/src/naming.rs`; the depth is the highest of every body's in the
  design plus one a feature, `eval.rs` `stamp`), and past it `Body:face:k`. A chamfer picked
  past it stores `Wall corner Cleat:face:3|...:face:5`; deleting an earlier cabinet brings it
  under 400, the face is `F662:+z` and the chamfer fails ("no longer exists as it was picked").
  The sample kitchen without fasteners had 419 features before the wall corner's bevel (not all
  count) and survived; with them everywhere it did not. So `examples/kitchen.py` sets Fastener
  models off before all but the units on the return (`LATE`), made after the wall corner.
  Fasteners everywhere cost the kitchen about 1.2 s on its slowest run (7.4 to 8.5 s).
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
- `plinth_type` (Woodwork settings and Edit; `fittings::plinth_type`, `board` when absent, so older cabinets keep theirs):
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
  list a rail bought to length and two hangers; no hanger models yet. The cut list gives
  the cleats as square boards (the bevel is sawn after).
  A corner cabinet gets them along both walls (`Cleat`, `Cleat return`, and the same for the
  wall cleats and rails; the return's bevel edges are the box's +x or -x ones, 11 and 0 or 7
  and 2): the backs stand the mount's thickness off the walls between bottom and top (`ech`
  high), bottom and top reach the walls (`w0` 0), the shelves and pin rows stop at the backs.
  Steel gives it `_wall_rail` and `_wall_rail_return` (`edit::SIZES` has both); the cut list reads
  either.

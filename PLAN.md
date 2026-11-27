# solidcraft-woodwork: plan

A SolidCraft plugin for designing cabinets and furniture in sheet goods and timber.

## How SolidCraft plugins work (0.41 to 0.45)

- A plugin is a folder with `plugin.json` (manifest) and one WebAssembly **component**.
- The component exports `execute(entry, params_json)` and can only call the app's normal
  commands (`solid.box`, `solid.hole`, ...) through `commands.run`. Same commands as MCP.
- The manifest lists entries (each with a JSON input schema, which the app draws as a
  dialog under the Plugins menu; 0.42 adds a ribbon button per entry), the exact commands
  used, and limits.
- One run is one undo step. The result is ordinary features, so a design opens without
  the plugin. No custom UI, no network. Files only in a folder the user approves at install
  (0.43).
- 0.44: the dialog previews an edit entry before Run, and the selection is a field in it.
- 0.45: each entry is also a tool for agents (`woodwork.<entry>`), a `read` entry may not
  write files, and `storage@0.1` can keep plugin data in a design (Woodwork settings, 0.46, with the dialog hook).
- Limits that shape the design: 256 app commands per run, 30 s, 1 MiB result, a manifest of
  at most 64 KiB.
- Install: zip of `plugin.json` + wasm, via Preferences > Plugins or `plugin.install`.

## Architecture (same as the gear and SolidInserts plugins)

- Rust, built for `wasm32-unknown-unknown`, wrapped with `wasm-tools component new`.
- Pure Rust modules turn parameters into a list of app commands. They are tested on the
  host with `cargo test`, without SolidCraft.
- A thin wasm guest runs the commands in order.
- An end-to-end script runs the built plugin through `solidcraft-cli` and checks the result.

## What other woodworking tools offer

Surveyed: FreeCAD Woodworking, OpenCutList (SketchUp), MapBoards Pro and ExportCutlist
(Fusion), SWOOD (SolidWorks), Woodwork for Inventor, Polyboard. Ranked by how common:

1. Cut list / BOM with CSV export
2. Sheet cutting diagrams (kerf, grain)
3. Material library (thickness, sheet size, grain)
4. Parametric carcass generator
5. Connectors that drill both parts (dowel, confirmat, minifix)
6. Edge banding per edge
7. Grain direction per part
8. Panel joinery (dado, rabbet, back groove)
9. Doors, drawers, shelves with hardware drilling (hinge cups, 32 mm shelf pins, slides)
10. Shop output (DXF per part, labels)

0.2.0 covers 1, 2, 4, 5, 6 and 7, and partly 3 (four materials), 8 (back groove
and dado, no rabbet), 9 (box cabinets only for slide and plate holes) and 10 (DXF per sheet, labels, no DXF per part).

## Done

- [x] `cabinet`: box carcass from width/height/depth and panel thickness. Sides, top,
      bottom, fixed shelves, back (none, overlay, or in a groove), optional plinth.
      Each panel is a named body.
- [x] `cutlist`: read the design and return every board as length x width x thickness,
      grouped with quantities, plus CSV text.
- [x] Shelf pin holes as a cabinet option: 32 mm system (5 mm holes, 37 mm setback), shelves rest on the pins
- [x] Doors as a cabinet option: one or two overlay doors with 35 mm hinge cup holes
- [x] Inches: a Units field on the cabinet, and the cut list follows the design's units
- [x] Vertical dividers: equal compartments with the same shelves in each
- [x] One door per compartment
- [x] Resizable: sizes are design parameters, the cabinet follows them
- [x] Each cabinet is a component (moves as one piece, nests in a room component)
- [x] Place a new cabinet right of, left of or above the selected one
- [x] Materials (per cabinet or selected parts) and a sheet estimate in the cut list
- [x] Joinery on the selected cabinet: confirmat or dowel holes at the carcass corners (removed: joinery is chosen at build; Edit gives an old drilled cabinet the same joint)
- [x] Doors and drawers are sub-components with hinge and slide joints
- [x] Drawers: a drawer unit with fronts and boxes sized for 12.7 mm slides
- [x] Hardware list in the cut list: hinges, slides, shelf pins, screws, dowels, handles
- [x] Handle holes in doors and drawer fronts: knob or 128 mm pull
- [x] Corner cabinet: L-shaped, left or right, with doors
- [x] Automatic names: a taken name gets the next free number
- [x] Match selected: a new cabinet's height, depth, thickness and plinth linked to the selected cabinet's parameters
- [x] Doors turn about their front edge, like a cup hinge, so they clear the next cabinet
- [x] Corner cabinet doors are a bi-fold pair, hinged to each other at the inner corner
- [x] Corner cabinet: handle holes, shelf pin holes, and a choice of which end the doors hang on
- [x] Diagonal corner cabinet: one front at 45 degrees with one door
- [x] Handle position on doors: top, middle, bottom or auto, for knobs and pulls on both cabinets
- [x] SolidCraft 0.42: ribbon buttons, tests run against 0.42
- [x] SolidCraft 0.43: every dropdown shows titles and an explanation per choice, tests run against 0.43
- [x] Dialog fields in a sensible order (sizes first) instead of alphabetical
- [x] Edit cabinet: change shelves, doors, drawers, handles and more on a cabinet that is already made
- [x] Labels: each board's name written on it
- [x] Corner cabinet: left or right follows where it is placed (Auto)
- [x] Box cabinet: the first door hinged left or right
- [x] A placed cabinet is held to its neighbour, so a run follows a change of width or height
- [x] Holes that survive a rebuild of the history: cut as cylinders (deleting a cabinet in a big design broke drilled holes)
- [x] SolidCraft 0.44: the selection is a field in the dialog, picked in the model; the cabinet is previewed before it is added; tests run against 0.44.1
- [x] An icon for each function, on the ribbon, in the menu and in its dialog
- [x] Cabinets in a run are held by their corners at the wall: backs stay flush and a corner cabinet's return takes its cabinets along, whichever size changes
- [x] Place: Auto, the default, by the selected board (left of a left side, right of a right side, above a top)
- [x] SolidCraft 0.45: the view is framed after a cabinet is added, each function is a tool for agents, the package is checked when built; tests run against 0.45.0
- [x] Inset doors and drawer fronts on the box cabinet, and a reveal (the gap round the fronts) as a parameter on both cabinets
- [x] Hinges that follow a resize: a door opens about its own edge after the cabinet changed size
- [x] Sink base: a cabinet kind with two rails instead of a top, no shelves
- [x] Sink base with drawers: a false front before the sink, half-depth boxes below (#55)
- [x] Cut list saved as a CSV file, in a folder approved at install (SolidCraft 0.43)
- [x] Cut list saved as a printable PDF, with tick boxes for boards and hardware
- [x] Edge banding and grain direction in the cut list
- [x] Sheet cutting diagrams (nesting, kerf, grain)
- [x] Part labels, a sticker per board (A4, 3 x 8)
- [x] DXF per sheet: parts, names and holes on a layer per diameter and depth
- [x] Shop drawings: a sheet per cabinet with front, right and iso views, sizes and the board list
- [x] Drawing a big design within the 30 s run limit: only changed sheets, more runs when needed
- [x] A tooltip on every dialog field and choice, in plain words
- [x] Version 0.2.0, for SolidCraft 0.45
- [x] SolidCraft 0.46: the dialogs hide the fields that do not apply (conditional fields)
- [x] Version 0.3.0, for SolidCraft 0.46
- [x] Drawers over or under doors, and a graduated drawer stack (the top drawer's height given, the rest equal) (#57 part 1)
- [x] Inset doors on corner cabinets: the L's bi-fold pair and the diagonal door inside the opening (#58)
- [x] Plinth types: a set-back board, notched sides (no board, for one long plinth across a run) or adjustable legs, on box and corner cabinets
- [x] Wall mount on box cabinets: French cleat with its wall cleat, hanging rail, or a steel rail listed with its hangers (#78; hanger models later, #70)
- [x] Wall mount on corner cabinets: cleats, hanging rails or steel rails along both walls, through the box cabinet's shared code (`fittings::wall_mount`, `mount_sizes`, `steel_rail`, `mount_board`)
- [x] Casters as a plinth type on box cabinets: locking casters with pilot holes for their plates, counted as hardware (#79)
- [x] Casters on corner cabinets, at the legs' places through the shared `fittings::foot`; the casters' post is joined before the wheel, so no timeline warning
- Frame (a leg-and-rail base) removed: not useful on its own; a different design is planned in #101. Its bolts and barrel nuts went with it (no cabinet joint used them)
- [x] Joinery chosen when a box cabinet is made, through one shared module (`src/joint/`): dowels (the default), confirmat screws, plain screws, cam locks, pocket screws, biscuits or Dominos, or a dado (box cabinets) where the sides meet the bottom and top, rows that follow a resize, counted in the cut list from parameters, changed by Edit, a default in Woodwork settings; the Frame's dowels and bolts on the same module
- [x] SolidCraft 0.47: tests run against 0.47.0
- [x] Version 0.4.0, for SolidCraft 0.47 (works on 0.46)
- [x] Version 0.5.0, for SolidCraft 0.49
- [x] Faster in a large design: placing, Edit, a panel, a worktop, Material and Apply settings measure only the cabinets they read (`inspect.measure` with `names_only`, then the bodies of those cabinets; a whole kitchen took over a second each time), and one pattern drills every panel's shelf-pin rows where their rows match (a feature less per panel)
- [x] Place Along a line (#147, step 1): its back on a picked sketch line, model edge or wall face, from either end or the middle with an offset, facing away from the wall or flipped; quarter turns only; Width from line follows a sketch line's driving length dimension by expression
- [x] Along a line, step 2 (#147): the cabinet grows away from its Start at end (its translation an expression over its own width and depth), Height from line (a second pick, following a sketch dimension), and Apply settings reading the remembered lines again (width, height and place), so an edge or undimensioned line is followed by a rebuild
- [x] Shared parameter groups for matched cabinets (#137): Match selected points every cabinet of a run at `Run_<first>_height` and the rest, so deleting a cabinet breaks none and changing one cabinet's own parameter takes only it out
- [x] Prices and a cost estimate (#56): Woodwork settings' Prices (`name = price` text), the cut list prices each sheet by material and thickness, the banding a metre and the hardware a piece (longest matching name), lists what has no price yet, and shows it in its reply, the PDF and the CSV

## Open

Cabinets:
- [x] Corner cabinet: joinery at build (L-shaped and diagonal, split boards, with a wall mount)
- [ ] Corner cabinet: a thin back
- [x] Joinery for dividers and the rail between drawers and doors (`joints_inner`, the second method)
- [x] Fixed shelves: without shelf pins, joined to the sides and dividers by the inside's method (box cabinets)
- [ ] A cabinet turned a quarter in the dialog, to continue a corner cabinet's return
      (today: turn the component afterwards)
- [x] Worktop over the selected cabinets: runs and corners (butt or mitre), gaps spanned, made again by running it again (#62)
- [x] Worktop dog holes (96 mm grid, all over or in a zone) and a plain or rebated cut-out (#79)
- [x] Worktop tool station: a benchtop tool in an opening, flush, on a shelf and battens in the cabinet below
- [ ] Plinth run over the selected cabinets: one long board across a run of cabinets with notched sides or on legs (Plinth type gives them no board of their own)
- [x] End panels and fillers on a cabinet's end, kept by Edit cabinet (#57 part 2)
- [ ] Plinth return: for cabinets on legs, later
- [x] Drawer slides drilled: screw holes in the box sides and the carcass sides, the box width from the slide thickness, all from shared `HW_` parameters (#59 part 1)
- [ ] Drawer slides in the model: slide length in the hardware list, undermount slides (#59)
- [x] Hinge plates drilled: the plates' screw holes in the sides, cups and plates from shared `HW_` parameters (#60 part 1)
- [x] Corner doors on the shared hinge holes: `HW_` cups on the pins' grid and plate holes in the end the pair hangs on (`hardware::cups`, `cup_holes`, `plate_holes`, also used by the box cabinet); the diagonal door gets cups only
- [x] Shared hardware models (#70): a simple hinge (cup and arm in the door, mounting plate on the side) in each cup and a slide's two members on each drawer, copies of definitions in `Woodwork hardware` at the root, moving with the doors and drawers; field Hardware models (shown, none, or cutting the cups for SolidCraft 0.47)
- [x] Drawer slide length: standard lengths in mm or inches, Auto the longest that fits, a field to choose one; the models, screw holes and the cut list's hardware line follow it
- [x] Slide type (partial, full extension, over-travel) and hinge opening (95, 110, 165 degrees) set the joints' limits; hinges per door by its height (2 to 5)
- [ ] Undermount slides (box width less 42, notches, rear hook holes), and the cup's screw holes
- [ ] Hinges in the model with hinge types (#60); hinge models that cut their cups by default once SolidCraft 0.47 is out (upstream tangency-labs/solidcraft#20787)
- [x] Woodwork settings: material, thickness, reveal, handles and plinth defaults kept in the design, which the Cabinet and Corner dialogs start from through the dialog hook (#61). Per design, not per user (upstream #19979); the last used values are not remembered (the hook cannot write)
- [x] Joinery and hardware are design wide: the two methods, their sizes, hardware models, hinge opening and slide type are Woodwork settings, no longer Cabinet and Corner fields (a new cabinet takes them and remembers them; Edit changes the methods, models, opening and type of one cabinet; slide length is Auto at build, chosen in Edit); the manifest 6.5 KB smaller
- [x] See-through boards (Woodwork settings): our board materials at 35 % opacity to look at the joinery and fittings inside, also materials made later; Oak (built into the app) stays solid
- [x] Fastener models: dowels, screws, cam locks with bolts and biscuits as bodies in their holes, one per joint and a pattern of bodies per row, hidden from every measure like the hinge models; Fastener models in Woodwork settings (off by default, for looking with See-through boards). Each joint adds a feature: past about 400 features SolidCraft 0.46 names faces another way and a later picked chamfer breaks on delete or Edit, so the sample kitchen shows them only on the return's units
- [x] More design-wide settings: back, fronts, handle position, the plinth (Cabinet keeps Yes or No), labels, cleat sizes, the cut list's banding, sizes, sheet, kerf and trim, a worktop's material and thickness
- [x] Woodwork settings: Apply to the picked cabinets, built again with the settings (as many as fit in a run, the rest the next run)
- [x] One Cabinet dialog for box and corner cabinets (Shape); the Corner cabinet entry is gone
- [x] Every dialog reviewed: field order (controllers before what they show), tooltips and placement
- [ ] Corner chamfers and cleat bevels without picked edges (a cut by a sketched profile), so a design past 400 features survives deleting an early cabinet
- Oven housing dropped by decision: size a tall cabinet for the oven yourself
- [ ] Dishwasher gap: an integrated front alone in a run, keeping the seats

Cut list and shop output:
- [ ] Cost estimate: prices per material and thickness, the cost of sheets, banding and hardware in the cut list (postponed until conditional dialog fields, SolidCraft 0.46, #56)
- [x] L-shaped corner cabinet's bottom, top and shelves split in two dowelled rectangles (`boards: split`), so they nest on a sheet (#63)
- [ ] Diagonal corner cabinet boards split in two (they keep their chamfer), and the L's backs over a sheet's width in grained material (#63)
- [ ] DXF per part (one file per board)

## Waiting for the app

The dialog is drawn by the app from a fixed schema. These need the `ui` capability that is planned
upstream (`storage`, from 0.45, keeps data in a design, not in the dialog):

- A dialog that stays open to add several cabinets, and remembers the last values

Possible since 0.44, not done:

- Pictures for choices such as a left or right corner (`x-solidcraft-icon` on a choice's
  options). Each costs manifest bytes; 0.45 allows 64 KiB.

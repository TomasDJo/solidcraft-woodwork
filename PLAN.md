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
  the plugin. No custom UI, no network, no file access without a grant (no grant UI yet).
- Limits that shape the design: 256 app commands per run, 30 s, 1 MiB result.
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
- [x] Joinery on the selected cabinet: confirmat or dowel holes at the carcass corners
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

## Open

Cabinets:
- [ ] Corner cabinet: joinery, a thin back
- [ ] Joinery for dividers and fixed shelves
- [ ] A cabinet turned a quarter in the dialog, to continue a corner cabinet's return
      (today: turn the component afterwards)
- [x] Hinges that follow a resize: a door opens about its own edge after the cabinet changed size
- [x] Worktop over a straight run of the selected cabinets (sink and hob cutouts dropped by decision: simple to cut afterwards)
- [ ] Drawers over doors and a graduated drawer stack (the top drawer's height given, the rest equal) (postponed until conditional dialog fields, SolidCraft 0.46)
- [ ] Plinth run over the selected cabinets
- [ ] End panels and fillers on a cabinet's end, kept by Edit cabinet (postponed until conditional dialog fields, SolidCraft 0.46)
- [x] Sink base: a cabinet kind with two rails instead of a top, no shelves
- Oven housing dropped by decision: size a tall cabinet for the oven yourself
- [ ] Dishwasher gap: an integrated front alone in a run, keeping the seats

Cut list and shop output:
- [x] Cut list saved as a CSV file, in a folder approved at install (SolidCraft 0.43)
- [x] Cut list saved as a printable PDF, with tick boxes for boards and hardware
- [x] Edge banding and grain direction in the cut list
- [x] Sheet cutting diagrams (nesting, kerf, grain)
- [x] Part labels, a sticker per board (A4, 3 x 8)
- [ ] Cost estimate: prices kept in the design, the cost of sheets, banding, worktops and hardware in the cut list (postponed until conditional dialog fields, SolidCraft 0.46)
- [x] DXF per sheet: parts, names and holes on a layer per diameter and depth
- [x] Shop drawings: a sheet per cabinet with front, right and iso views, sizes and the board list
- [ ] DXF per part (one file per board)

## Waiting for the app

The dialog is drawn by the app from a fixed schema. These need the `ui` capability that is planned
upstream (`storage`, from 0.45, keeps data in a design, not in the dialog):

- A dialog that stays open to add several cabinets, and remembers the last values
- Fields that depend on each other (defaults in inches, options that hide)

Possible since 0.44, not done:

- Pictures for choices such as a left or right corner (`x-solidcraft-icon` on a choice's
  options). Each costs manifest bytes; 0.45 allows 64 KiB.

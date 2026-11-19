# Using Woodwork

The functions are under **Plugins > Woodwork** and as buttons on the ribbon. Each opens a
dialog; the cabinet you are about to make is previewed in the model before you press Run.

- [Cabinet](#cabinet)
- [Corner cabinet](#corner-cabinet)
- [Building a run: Place and Match selected](#building-a-run-place-and-match-selected)
- [Changing sizes](#changing-sizes)
- [Edit cabinet](#edit-cabinet)
- [Joinery](#joinery)
- [Material](#material)
- [Worktop](#worktop)
- [Cut list](#cut-list)
- [Prices](#prices)
- [Good to know](#good-to-know)

## Cabinet

Builds a box carcass: sides, top, bottom, shelves, back and optional plinth. Each panel is an
ordinary named body. The cabinet stands on Z = 0, centred on the origin, and is a component
of its own inside whichever component is active, so it moves as one piece and a room
component can hold its cabinets.

| Field | What it does |
|---|---|
| **Units** | The sizes in the dialog in mm or inches. |
| **Width, Height, Depth, Panel thickness** | The outside sizes. Height includes the plinth. |
| **Back**, **Back thickness** | In a groove, overlaid on the rear edges, or none. |
| **Dividers** | Vertical dividers, splitting the inside into equal compartments, each with the same shelves. |
| **Shelves** | Shelves in each compartment, evenly spaced. |
| **Drawers** | Makes it a drawer unit: a stack of equal drawers with overlay fronts and boxes sized for 12.7 mm ball-bearing slides. |
| **Plinth height** | 0 for none. |
| **Shelf pin holes** | 32 mm system rows of 5 mm holes; the shelves rest on them. |
| **Doors** | Overlay doors with 35 mm hinge cup holes: one, two, or one per compartment. |
| **First door hinged** | The side the first door is hinged on. The next ones alternate, and two doors always open from the middle. |
| **Handle holes** | Drills the doors and drawer fronts for a knob (one 5 mm hole) or a pull (two, 128 mm apart). |
| **Handle position** | Near the top, in the middle or near the bottom of a door. On Auto: near the top on a floor unit, near the bottom on a wall unit. On a drawer front the handle lies across: in the middle on Auto, or 40 mm from the front's top or bottom edge. |
| **Labels** | Writes each board's name on it, the name the cut list has, as text in a sketch on the board's face. It stays in the middle of the board when the cabinet changes size. |
| **Selected cabinet**, **Match selected**, **Place** | See [Building a run](#building-a-run-place-and-match-selected). |
| **Material** | The board material of every panel. Change it later with [Material](#material). |
| **Name** | The prefix of the panel names. A name that is taken is numbered: Cabinet, Cabinet 2, Cabinet 3; after Base 1 comes Base 2. |

### Doors and drawers move

Every door and every drawer is a component of its own inside the cabinet, held by a joint:
doors hinge open to 110 degrees, drawers slide out. Drive the joints to open them. A door
turns about its front edge, as on a cup hinge, so it clears the door of the next cabinet.

The cabinet is grounded so they have something fixed to open against; unground it to move
it. A cabinet placed against another is held to that one instead.

## Corner cabinet

Builds a cabinet for a corner of the room: bottom, top and shelves that fill the corner,
optional plinth, shelf pin holes, and doors hinged at the end you choose, with handle holes.
It is parametric, takes a material, and can be placed, named and matched like the box cabinet.

**Corner** is the side the return is on. On Auto it is on the right when you place the
cabinet right of the selected one, as a run that ends in a corner needs it, and else on the
left.

Two shapes:

- **L-shaped**: two fronts at a right angle, with a bi-fold pair of doors. The first door is
  hinged on the cabinet and closes the inner corner; the second hangs on it there and folds
  out onto its face, so the two swing out together. The handle is on the second door, at the
  position you choose.
- **Diagonal**: one front straight across the corner at 45 degrees, with one door. The depth
  at the end of the return follows from the width and the return, and equals Depth when the
  two are the same.

## Building a run: Place and Match selected

Select any part of a cabinet, then open **Cabinet** or **Corner cabinet**. The selection
shows in the dialog as **Selected cabinet**. Click that field and pick another board in the
model while the dialog is open, and the preview follows. Edit cabinet, Joinery and Material
have the same field.

**Place** puts the new cabinet against the selected one, backs flush. That cabinet must
stand square to the axes.

| Place | Where the new cabinet goes |
|---|---|
| **Auto** (the dialog opens with it) | By the board you selected: left of a left side, right of a right side, above a top. At the origin with nothing, or any other board, selected. |
| **At the origin** | Centred on the origin. |
| **Right of selected**, **Left of selected** | Against that cabinet's right or left side, whichever way it stands. At the end of a corner cabinet's return the new one is turned a quarter and continues along the return, and the run carries on from there, corner cabinets too. |
| **Above selected** | On top of it. |

**A run stays together.** The new cabinet is held to its neighbour by a joint (`Name link`):

- Make a cabinet wider and the ones placed after it move along.
- Make one deeper and the backs stay flush.
- Make a corner cabinet wider, or its return longer, and the cabinets along the return move
  with it, their backs still on the wall.
- Make one higher and the one on top of it rises. A cabinet on top follows upwards only.
- The first cabinet of a run stands on its own middle, so it grows to both sides.
- To take a cabinet out of a run, delete its link.

**Match selected** links the new cabinet's height, depth, panel thickness and plinth to the
selected cabinet, so a whole run is resized by changing one cabinet's parameters.

## Changing sizes

A cabinet's sizes are design parameters: `Name_width`, `Name_height`, `Name_depth`,
`Name_thickness` and so on. Change one in the Parameters table and the cabinet follows, its
doors' hinges included.

The counts (shelves, dividers, doors, drawers) are changed with Edit cabinet.

## Edit cabinet

Changes a cabinet after it is made. Select any part of it, fill in only what should change
and leave the rest empty or on Keep: shelves, dividers, doors, the side the first door is
hinged on, drawers, back, shelf pin holes, handles, plinth, a corner cabinet's shape.

The cabinet is built again where it stands, under the same name. Its sizes stay as they are
in Parameters, its doors keep their material, its joinery is drilled again, and cabinets
matched to it still follow. Doors and drawers come back closed.

Works on cabinets made by 0.1.7 or later.

## Joinery

Drills the selected cabinet where its sides meet the bottom and top: holes for 7 x 50
confirmat screws, or for 8 x 35 dowels. The holes follow the cabinet's parameters; their
number is set when they are drilled.

## Material

Gives the selected cabinet, or only the selected parts (the doors, say), a board material:
birch plywood, MDF, white melamine or oak. New cabinets get one in their dialog.

## Worktop

Lays one board over a straight run of cabinets. Select a part of each cabinet of the run (the
**Selected cabinets** field takes several), then open **Worktop**. The cabinets must stand
side by side with no gap, backs flush, facing the same way, on the same floor and equally high;
a corner cabinet may be at either end, and the leg that lies along the run counts.

| Field | What it does |
|---|---|
| **Units** | The sizes in the dialog in mm or inches. |
| **Thickness** | Of the board. |
| **Front overhang** | Past the carcass front of the deepest cabinet; doors and drawer fronts stand about 20 mm proud, so the default 40 leaves 20 mm past them. The back is flush with the cabinets' backs. |
| **Left end overhang**, **Right end overhang** | Past the run's ends, left and right as seen from the front. |
| **Material** | The board's material. |
| **Name** | The board's name. A name that is taken is numbered: Worktop, Worktop 2. |

The worktop is a component of its own, grounded, with its sizes as parameters: `Worktop_width`
is the sum of the cabinets' widths plus the end overhangs, `Worktop_depth` the deepest
cabinet's depth plus the front overhang, `Worktop_thickness`, and it sits on the cabinets'
height. Make a cabinet wider, deeper or higher and the worktop follows. Cut a sink or hob hole
yourself afterwards. Edit cabinet leaves it alone.
It holds where the run holds: on the middle of a grounded cabinet of the run (which grows to
both sides), or at the side by which the run is held to a cabinet outside it, so a run built
leftwards grows to the left with it. A cabinet that is ungrounded, linked or moved after the
worktop was made is not followed, and only the deepest cabinet at that time sets its depth.

To change a worktop afterwards (Edit cabinet does not take it): delete its component and make
it again, or change `Worktop_thickness`, and the overhang terms in the expressions of
`Worktop_width` and `Worktop_depth`, in the Parameters table.

**An L-shaped kitchen** gets two worktops, one per straight run. The corner cabinet belongs to
one of them; the other run's worktop butts against the first one's front edge, with its end
overhang at the corner set to the corner cabinet's other leg less its depth less the first
worktop's front overhang. For a 1000 mm return, 560 deep, under a worktop with 40 mm front
overhang: 1000 - 560 - 40 = 400 mm. Which end that is goes by the front of the run: for a run
along the return of a right-hand corner, the corner is at its left end.

## Cut list

Returns every board as length x width x thickness with quantities, and as CSV. In a design
set to inches or feet it lists inches as fractions to 1/32. It also:

- lays the parts out on sheets per material and thickness, and counts the sheets to buy from
  those layouts (see [Cutting layouts](#cutting-layouts));
- counts the hardware: hinges, drawer slides, shelf pins, screws or dowels, handles;
- marks the edges to band, as OpenCutList does: `L2 W2` is both edges along the length and both
  along the width, `L1` one edge along the length. Doors and drawer fronts are banded all round; sides, top,
  bottom, shelves and dividers at the front; backs, plinths and drawer boxes not. It adds up
  the metres of banding per material and thickness, plus 10 % waste;
- adds up what it costs, when the design has prices (see [Prices](#prices) and
  [Cost estimate](#cost-estimate));
- lists a board of a material with a grain (Birch plywood, Oak) with its length along the
  grain, marked `grain`: up on sides, doors, backs, dividers and drawer fronts, along the
  cabinet's width on top, bottom, shelves and drawer bottoms, and along the drawer box's
  sides, front and back. Such a part must not be turned on the sheet. Other materials (MDF,
  White melamine) have the longest side as the length, and so do plinths and the drawer
  box's sides, front and back.

**Edge banding** is the tape's thickness: none, 0.4, 1 (the default) or 2 mm. **Sizes**:
finished, as in the model, or cut, the size to saw: the finished size less the tape on each
banded edge. The list says which it shows.

Fill in **Save as** with a name to also get the list as a file, in the **Format** chosen:
PDF, a printable A4 list with a tick box per board and per piece of hardware, one table per
material and thickness, the sheets to buy and the cutting diagrams; CSV, for a spreadsheet; or
Labels (PDF), a sticker per board to stick on after cutting (see [Part labels](#part-labels)); or
DXF, a drawing per sheet for a CNC router or CAD program (see [DXF](#dxf)). It goes into the folder for cut
lists that SolidCraft asks you for when you install the plugin; without such a folder, leave
the field empty.

### Cutting layouts

The parts are laid out on sheets of the **Sheet size** chosen (2440 x 1220, the default,
2500 x 1250, 2800 x 2070 or 3050 x 1530 mm), with the **Saw kerf** (4 mm) left between parts
and the **Edge trim** (10 mm) cut off every edge of the sheet. Each layout can be cut on a panel
saw: every cut runs right across the piece it is made in. A part of a material with a grain
keeps its length along the sheet's length; any other part may be turned. Parts are laid out at
the size the list shows, so at the cut size when **Sizes** is Cut. A part that is too big for
the sheet less its trim (or fits only with its grain across the sheet) is not laid out: the
reply lists it under `oversize` with the reason, and the PDF names it under the sheets to buy.
Sheet sizes, kerf and trim are metric mm: a 4 x 8 ft sheet is 2438.4 x 1219.2 mm, so pick
2440 x 1220 and leave the trim on.

A worktop made by the Worktop function, whatever its name, or a board whose name contains
`Worktop`, is not sheet stock: it is left out of the layouts and
the sheets, and listed as bought to length (the reply's `worktops`, and a table "Worktops,
bought to length" in the PDF). It still appears in the cut list's rows.

The layouts are a quick greedy fit, not an optimiser: a few orders and two ways of choosing
the free piece are tried and the one with the fewest sheets kept. A cutting program may well
save a sheet on a big job.

**Cutting diagrams** (Yes, the default, or No) adds a drawing of each sheet to the PDF, after
the tables: per material and thickness a legend numbering the parts as rows of the list (equal
parts share a number), then each sheet with its parts outlined and numbered with their size
(the number only on a small part), the offcuts grey and the trim dashed. Each sheet's heading
gives the material, thickness, sheet n of m and its yield, the share of the sheet that becomes
parts.

### Part labels

**Format** Labels (PDF) saves `<name>.pdf` with one label per board (a part of quantity 2 gets
two), on A4 sheets of 3 x 8 labels of 70 x 37 mm without margins (Herma or Avery 3474), with
thin lines where the labels meet. Each label has the board's name, its size (length x width x
thickness, as the list shows it), its material, its number and sheet in the cutting diagrams
(`#12, sheet 3 of 5`, sheets counted per material and thickness), and a small drawing of the
part, length across: its banded edges thick (named too, `edges L1`) and an arrow along the
length when it has a grain. The labels follow the cutting diagrams sheet by sheet, so a
sheet's parts come off it in label order; boards on no sheet come last, marked "bought to
length" (worktops) or "not on a sheet" (too big). A long name is set smaller, down to 7 pt,
then cut short with `...`.

### DXF

**Format** DXF saves one file per sheet of the cutting layouts, `<name>-<material>-<thickness>-<n>.dxf`
(`kitchen-White melamine-18-3.dxf`, n counting the sheets of each material and thickness from 1),
and the reply's `saved` lists them all. Each is an ASCII DXF R12 with every size in mm, x along the sheet's
length and y across it, as the cutting diagrams, on these layers:

- `SHEET`: the sheet's outline, and the edge trim inside it
- `OUTLINE`: each part's rectangle, where it lies on the sheet
- `TEXT`: each part's name in its middle, along its longer side
- `DRILL_<diameter>_D<depth>`, or `DRILL_<diameter>_THRU` for a hole right through: the holes
  drilled into the part's face, in mm to 0.1 with a `P` for the decimal point (`DRILL_5_D12`
  shelf pins, `DRILL_35_D13` hinge cups, `DRILL_5_THRU` handle screws, `DRILL_4P5_D10P9`). A hole drilled from the other face than the one drawn is
  on the same layer with `_BACK` added. Each part is drawn from the face with the most holes.

The holes are read from the model, one command per drilled board (about 60 in the sample
kitchen). Limits: a hole drilled into an edge (confirmat pilot, dowel into a board's end) cannot
be drawn on a sheet and is only counted, in the reply's `dxf_edge_holes`; grooves (the back
panel's) are not drawn; at Cut sizes a hole is placed from the part's middle, so up to half the
banding (1 mm) off toward a banded edge; a square or nearly square part may get its holes
turned a quarter against its grain, as its length is told from its width by size. A design with more drilled boards than a run's 256
commands allow gets outlines only for the rest, named in `dxf_holes_skipped`. A sketch's
**Insert DXF** reads the files back.

For scripts the reply has, beside the rows: `sheets` per material and thickness (`area_m2`,
`sheets` to buy, `yield_pct`), `sheet_mm` (length and width), `kerf_mm`, `trim_mm`,
`layouts` (one per sheet: `material`, `thickness`, `size`, `yield_pct` and `parts`, each with
`part`, `row` (its row in `parts`, from 0), `x`, `y`, `w`, `h` and `rotated`; x along the
sheet's length from its corner; when saving DXF also `holes`, each `[x, y, diameter, depth or
null, back]` on the sheet), `oversize` (`part`, `material`, `thickness`, `length`, `width`
in mm and `reason`) and `worktops` (`part`, `material`, `thickness`, `length`, `width` as the
list shows them). Layouts are always in mm (`layout_units`), also in a design in inches. In a
design in inches each of `sheets` also has `thickness_mm` and each of `worktops` `length_mm`,
the mm the cost estimate goes by.

### Cost estimate

When the design has prices (set with [Prices](#prices)), the cut list adds a cost estimate: a
line for each sheet stock (sheets to buy x the sheet's price), the edge banding (metres with
waste x the price per metre of the tape chosen in **Edge banding**), each worktop (its length x
the price per metre) and each piece of hardware (quantity x price), and the total. A line
without a price, such as a material that is not one of the plugin's four or a price never
entered, is listed as "no price" and left out of the total, and the list says how many there
are. It is in the reply (`cost`: `currency`, `lines` with `item`, `qty`, `unit`, `price` and
`amount`, null when unpriced, `total`, `unpriced` and a `note`), in the PDF as a section after
the hardware, and at the end of the CSV. Without any prices there is no cost section, and the
reply has a `hint` that Prices sets them.

## Prices

Sets the prices the cut list's cost estimate uses. They are kept in the design (it remembers
them when saved and opened again, also on another computer) and are not shared between designs.

| Field | What it is |
|---|---|
| **Birch plywood, MDF, White melamine, Oak sheet** | The price of one 18 mm sheet of the size the cut list lays out on. Other thicknesses are priced by thickness: a 6 mm sheet costs a third. |
| **Banding 0.4, 1, 2 mm** | Per metre of tape; the cut list's **Edge banding** field picks which. |
| **Worktop** | Per metre of length. |
| **Confirmat screw, Dowel, Hinge, Knob, Pull, Shelf pin, Drawer slides** | Each; drawer slides per pair. |
| **Currency** | Shown with the prices, such as EUR. Up to 8 characters. |

Fill in only what should change: an empty field keeps the price stored before, and 0 is a price
of 0. Run shows the prices now stored. The dialog cannot show the stored prices in its fields,
and a price cannot be removed again, only changed. The fields come in alphabetical order
(the dialog orders fields it does not require by name).

There is one price per material, for one sheet size: if you change **Sheet size** in the cut
list, change the sheet prices too. Banding and worktops are priced per metre also in a design in
inches.

## Good to know

- **After a cabinet is added or edited the view shows the whole model** (SolidCraft 0.45), and
  nothing is selected: pick the board to place the next cabinet against.
- **An AI agent can use every function as a tool** (SolidCraft 0.45): `woodwork.cabinet`,
  `woodwork.corner` and so on, with the dialog's fields as its arguments. Every field is
  required there, so the agent sends them all, the defaults with what it wants changed.
- **A cabinet made by 0.1.7 or earlier** in a run is held along the run only: its neighbours
  do not follow its depth.
- **A design made with the plugin is ordinary bodies, joints and parameters.** It opens in
  any SolidCraft from 0.41 on, with or without the plugin.

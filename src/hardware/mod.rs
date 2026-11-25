//! The hinges' and drawer slides' sizes, shared by every box cabinet as design parameters
//! (`HW_cup_d`, ...), made the first time a cabinet needs them. Every hole a hinge or a slide
//! needs is a cylinder cut whose size and place are expressions over them, so changing one in
//! Parameters cuts every board again.
//!
//! A hinge is a cup in the door (`HW_cup_inset` in from its hinge edge) and a plate's two
//! screw holes in the side it hangs on (`HW_pitch` apart about the cup's height, `HW_setback`
//! behind the front of the opening, on the shelf pins' line). A slide is two screw holes in a
//! drawer box's side (`HW_setback` and whole `HW_pitch`es further, as the slide's length allows,
//! behind the box's front) and two in
//! the carcass side beside it (as far behind the front of the opening), in the middle of the
//! box side's height. The box is `2*HW_slide_t` narrower than the opening.
//!
//! The models (`hardware_models`, on by default) only show: a simple hinge (a cup and an arm, and
//! a mounting plate on the side) and a simple slide (a cabinet member and a drawer member), each a component defined once
//! per design under `HOME` at the root and placed by `occurrence.copy`: a hinge in its door's
//! component and a slide's two members in the drawer's and the cabinet's, so they move with
//! what they hold. The holes stay cylinder cuts, except with `cut`, where the hinge copies
//! cut the cups (`solid.combine`): slow on SolidCraft 0.46.1, where every later run
//! re-evaluates such cuts (upstream fix in 0.47).

use crate::fittings::{self, CUP_CENTRE, CUP_D, CUP_DEPTH, PITCH};
use crate::kit::{check, mm, Kit};
use crate::Commands;
use serde_json::{json, Value};

/// The shared sizes: name, value in mm, comment.
pub const PARAMETERS: [(&str, f64, &str); 10] = [
    ("HW_cup_d", CUP_D, "hinge cup diameter"),
    ("HW_cup_depth", CUP_DEPTH, "hinge cup depth"),
    (
        "HW_cup_inset",
        CUP_CENTRE,
        "hinge cup centre, in from the door's edge",
    ),
    (
        "HW_screw_d",
        5.0,
        "screw holes in the sides, for hinge plates and slides",
    ),
    (
        "HW_screw_depth",
        10.0,
        "depth of the screw holes in the sides",
    ),
    (
        "HW_setback",
        37.0,
        "screw holes for hinge plates and slides, behind the front",
    ),
    ("HW_pitch", PITCH, "between screw holes: the 32 mm system"),
    ("HW_slide_t", 12.7, "drawer slide thickness, each side"),
    ("HW_box_screw_d", 3.0, "screw holes in a drawer box's sides"),
    (
        "HW_box_screw_depth",
        8.0,
        "depth of the screw holes in a drawer box's sides",
    ),
];

/// The commands that make the shared parameters the design does not have yet. `design` is
/// `document.inspect`'s reply.
pub fn parameters(design: &Value) -> Commands {
    let params = design["params"].as_array().into_iter().flatten();
    let have: Vec<&Value> = params.map(|p| &p["name"]).collect();
    (PARAMETERS.iter())
        .filter(|(name, ..)| !have.contains(&&json!(name)))
        .map(|(name, value, comment)| {
            let args = json!({"name": name, "expression": mm(*value), "comment": format!("Hardware: {comment}")});
            ("parameters.add", args)
        })
        .collect()
}

// ponytail: the grid is `HW_pitch` and the plates stand `HW_setback` behind the front; the pin
// rows' are `fittings::PITCH` and `PIN_SETBACK`: they part when either is changed in Parameters.
/// A hinge cup's height `z` moved to the nearest place where its plate's two screw holes,
/// `HW_pitch` apart about it, are on the shelf pins' grid: whole pitches above `base`, the
/// bottom panel's top face (or the rail's, under doors over drawers). At most half a pitch.
pub fn on_grid(z: &str, base: &str) -> String {
    format!("{base} + HW_pitch/2 + round(({z} - ({base}) - HW_pitch/2) / HW_pitch) * HW_pitch")
}

/// The hinge cups' heights on a door from `z0` up to `z1`, `door_h` mm high
/// (`fittings::cup_heights`), each moved onto the pins' grid over `base` (`on_grid`).
pub fn cups(z0: &str, z1: &str, door_h: f64, base: &str) -> Vec<String> {
    let cups = fittings::cup_heights(z0, z1, door_h);
    cups.iter().map(|z| on_grid(z, base)).collect()
}

/// A door's hinge cups (`<Door> hinge cup n`), `HW_cup_d` wide and `HW_cup_depth` deep, at
/// `at(z)` on its back face for each height in `cups`, `into` the door (`toward` its hinge
/// edge, from the cup), and a hinge model in
/// each (`copy`, its arm cranked `crank` mm: `hinge`; with `cut` the model cuts the cup). `door` is the part: its component and its
/// body are `<Cabinet> <door>`.
pub fn cup_holes(
    kit: &Kit,
    door: &str,
    at: impl Fn(&str) -> Value,
    [into, toward]: [[i32; 3]; 2],
    crank: Option<f64>,
    cups: &[String],
) {
    let component = format!("{} {door}", kit.name);
    // ponytail: no model (`crank` None) for a corner's fold, a door hung on a door: its cups
    // are drilled, but a door-to-door hinge (plate on the other door, no holes for it) is not
    // modelled.
    let cut = kit.models == "cut" && crank.is_some();
    for (j, z) in cups.iter().enumerate() {
        let what = format!("hinge cup {}", j + 1);
        if !cut {
            fittings::hole(kit, door, at(z), into, "HW_cup_d", "HW_cup_depth", &what);
        }
        if let Some(crank) = crank {
            let name = cut.then(|| format!("{component} {what}"));
            copy(kit, &hinge(crank), &component, at(z), [into, toward], name);
        }
    }
}

/// The plates' two screw holes for each cup in `cups` (`<Board> hinge plate n.k`), `HW_pitch`
/// apart about its height, at `at(z)` on the inner face of `board`, `t` thick, `into` it; and
/// the hinge's mounting plate model over them, in the cabinet's component.
pub fn plate_holes(
    kit: &Kit,
    board: &str,
    at: impl Fn(&str) -> Value,
    into: [i32; 3],
    cups: &[String],
    t: &str,
) {
    for (j, z) in cups.iter().enumerate() {
        for (k, half) in ["-", "+"].iter().enumerate() {
            let position = at(&format!("{z} {half} HW_pitch/2"));
            let what = format!("hinge plate {}.{}", j + 1, k + 1);
            fittings::hole(
                kit,
                board,
                position,
                into,
                "HW_screw_d",
                &screw_depth(t),
                &what,
            );
        }
        // the hinge's mounting plate, over the two, fixed in the cabinet
        copy(kit, PLATE, &kit.name, at(z), upright(into), None);
    }
}

/// How deep a screw hole goes into a carcass side `thickness` thick: `HW_screw_depth`, but
/// never closer than 3 mm to its outer face.
pub fn screw_depth(thickness: &str) -> String {
    format!("min(HW_screw_depth, {thickness} - 3 mm)")
}

/// The component at the design's root that holds the models' definitions. A definition inside
/// a cabinet would be deleted with it, and every copy of it elsewhere.
pub const HOME: &str = "Woodwork hardware";
/// The models: each a component in `HOME` with one body of the same name. A hinge is a cup
/// and an arm, in the door (`HW hinge`, or with its arm cranked: `HW hinge crank 18`), and a
/// mounting plate, on the side; a slide is a cabinet member, a middle member (3-section
/// slides) and a drawer member, one definition each per length used (`HW slide drawer 450
/// mm`). Every model's name starts with `HW hinge` or `HW slide`.
pub const HINGE: &str = "HW hinge";
pub const PLATE: &str = "HW hinge plate";
const SLIDE: &str = "HW slide";
pub const SLIDE_CABINET: &str = "HW slide cabinet";
pub const SLIDE_MIDDLE: &str = "HW slide middle";
pub const SLIDE_DRAWER: &str = "HW slide drawer";
/// An undermount slide's runner under each box side (`HW slide under 450 mm`).
pub const SLIDE_UNDER: &str = "HW slide under";
/// The runner's width and height (unconfirmed, from Blum's sheets), and how far the box's
/// bottom stands above the sides' lower edge for it.
pub const UNDER_SIZE: [f64; 2] = [12.0, 14.0];
pub const UNDER_RECESS: f64 = 13.0;
// ponytail: `browser.visibility` set in a plugin run is not kept (0.46.1; the same command
// through the CLI is), so the definitions stand just under the floor (their tops 1 mm below
// it), turned half round so that they reach forward from 15 mm in front of the back of a
// 560 mm deep cabinet on the origin: under the first cabinet's back, away from the fronts
// that are looked at through see-through boards. No view from above sees them (from below
// they show), and Fit frames about 3 % wider than without them. A slide longer than 530 mm
// reaches out in front. A hidden flag upstream would do better.
/// Where `HOME` stands: a row-major 3 x 4 placement, half a turn about Z.
const HIDDEN: [f64; 12] = [
    -1.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0, 265.0, 0.0, 0.0, 1.0, -25.0,
];
const ORIGIN: [f64; 12] = [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0];

// The models' sizes in mm, from a common hinge (35 mm cup, 0 mm cruciform plate on the 37/32
// holes) and a 45 mm ball-bearing slide; they only show, so they are not parameters.
/// The plate: thickness off the side, depth along it (centred on the screw holes' line),
/// height.
pub const PLATE_SIZE: [f64; 3] = [8.5, 40.0, 48.0];
/// The arm: thickness off the plate, length back from the door's back face, height.
pub const ARM_SIZE: [f64; 3] = [11.0, 52.0, 16.0];
/// How far the cup's centre is inside the side's inner face on a full overlay door (22.5 mm
/// in from the edge, 18 mm sides, 2 mm reveal): where a straight arm's plate is. A cranked arm
/// (9.5 for half overlay, on a divider; 18 for inset) reaches that much further.
const ARM_REACH: f64 = 6.5;
/// The slide's members, outside in: name, thickness, height. Their thicknesses fill
/// `HW_slide_t` (12.7 mm) about; the middle one sits half way.
pub const MEMBERS: [(&str, f64, f64); 3] = [
    (SLIDE_CABINET, 6.0, 45.0),
    (SLIDE_MIDDLE, 3.0, 40.0),
    (SLIDE_DRAWER, 4.0, 35.0),
];

/// The hinge model for an arm cranked `crank` mm: 0 (straight, full overlay), 9.5 or 18.
pub fn hinge(crank: f64) -> String {
    match crank {
        0.0 => HINGE.into(),
        _ => format!("{HINGE} crank {crank}"),
    }
}

/// The slide types, as the `slide_type` field names them, and as the cut list does.
pub const SLIDE_TYPES: [(&str, &str); 6] = [
    ("partial", "partial extension"),
    ("full", "full extension"),
    ("over", "over-travel"),
    ("wood", "dado runners"),
    ("under_partial", "undermount partial extension"),
    ("under_full", "undermount full extension"),
];

/// How far a drawer on slides `length` mm long of `kind` (`SLIDE_TYPES`) pulls out, in mm: a
/// 2-section slide about three quarters of its length, a 3-section one its length, and an
/// over-travel one 25 mm more.
pub fn travel(kind: &str, length: f64) -> f64 {
    match kind {
        "partial" | "under_partial" => (length * 0.75).round(),
        "over" => length + 25.0,
        _ => length,
    }
}

/// The standard side-mount ball-bearing slide lengths, in mm and in inches, as the
/// `slide_length` field names them, and in mm.
pub const SLIDES: [(&str, f64); 20] = [
    ("250 mm", 250.0),
    ("270 mm", 270.0),
    ("300 mm", 300.0),
    ("350 mm", 350.0),
    ("400 mm", 400.0),
    ("450 mm", 450.0),
    ("500 mm", 500.0),
    ("550 mm", 550.0),
    ("600 mm", 600.0),
    ("650 mm", 650.0),
    ("10 in", 254.0),
    ("12 in", 304.8),
    ("14 in", 355.6),
    ("16 in", 406.4),
    ("18 in", 457.2),
    ("20 in", 508.0),
    ("22 in", 558.8),
    ("24 in", 609.6),
    ("26 in", 660.4),
    ("28 in", 711.2),
];
/// The standard undermount slide lengths (Blum TANDEM and the like): the box is as long.
pub const UNDER: [(&str, f64); 14] = [
    ("250 mm", 250.0),
    ("270 mm", 270.0),
    ("300 mm", 300.0),
    ("350 mm", 350.0),
    ("400 mm", 400.0),
    ("450 mm", 450.0),
    ("500 mm", 500.0),
    ("550 mm", 550.0),
    ("600 mm", 600.0),
    ("9 in", 228.6),
    ("12 in", 304.8),
    ("15 in", 381.0),
    ("18 in", 457.2),
    ("21 in", 533.4),
];
/// Room a slide leaves behind it in the drawer box's depth (an undermount one in the inside).
const SLIDE_CLEAR: f64 = 3.0;

/// The drawer slide's length, in mm and as a length with its unit (`450 mm`, `18 in`):
/// `field` is `slide_length`, `auto` (the longest standard length of the cabinet's `unit` that
/// fits the box, `box_mm` deep, less `SLIDE_CLEAR`) or one of the standard lengths.
pub fn slide_length(field: &str, unit: &str, box_mm: f64) -> Result<(f64, &'static str), String> {
    lengths(&SLIDES, field, unit, box_mm)
}

/// `slide_length` from the undermount lengths (`UNDER`), `room_mm` the cabinet's inside depth.
pub fn under_length(field: &str, unit: &str, room_mm: f64) -> Result<(f64, &'static str), String> {
    lengths(&UNDER, field, unit, room_mm)
}

fn lengths(
    all: &[(&'static str, f64)],
    field: &str,
    unit: &str,
    box_mm: f64,
) -> Result<(f64, &'static str), String> {
    let room = box_mm - SLIDE_CLEAR;
    if field == "auto" {
        let mut ours = all.iter().filter(|(label, _)| label.ends_with(unit));
        let shortest = ours.clone().next().map_or("", |s| s.0);
        let fits = ours.rfind(|(_, v)| *v <= room + 1e-9);
        return fits.map(|&(label, v)| (v, label)).ok_or(format!(
            "the drawer boxes are {box_mm:.0} mm deep: no standard slide fits (the shortest is {shortest}, with {SLIDE_CLEAR} mm behind it)"
        ));
    }
    let &(label, v) = (all.iter())
        .find(|(label, _)| *label == field)
        .ok_or("slide_length must be auto or a standard length such as 450 mm or 18 in")?;
    check(
        v <= room + 1e-9,
        &format!("a {label} slide does not fit drawer boxes {box_mm:.0} mm deep: pick a shorter one, or Auto"),
    )?;
    Ok((v, label))
}

/// Whether a body (`inspect.measure`'s name) is a model's definition or a copy of one, such as
/// `HW hinge (Base:1/Base Door:1/HW hinge:2)` or `HW slide drawer 450 mm`.
/// A joint's fastener (`<Name> joint ... fastener`, its pattern's copies `... fastener (2)`) is
/// one too.
pub fn is_model(body: &str) -> bool {
    let copy = body
        .rsplit_once(" (")
        .filter(|(_, n)| n.trim_end_matches(')').parse::<u32>().is_ok());
    let body_of = copy.map_or(body, |(name, _)| name);
    [HINGE, SLIDE].iter().any(|m| body.starts_with(m)) || body_of.ends_with(" fastener")
}

/// Takes the models out of `inspect.measure`'s reply: they are not boards, nor part of any
/// cabinet's footprint. Every function reads its bodies through this (`lib.rs` `execute`).
pub fn hide(measure: &mut Value) {
    if let Some(bodies) = measure["bodies"].as_array_mut() {
        bodies.retain(|b| !b["name"].as_str().is_some_and(is_model));
    }
}

/// The models a builder's commands copy, each once.
pub fn wanted(commands: &Commands) -> Vec<String> {
    let mut models = Vec::new();
    for (command, args) in commands {
        let model = args["component"].as_str().unwrap_or_default().to_string();
        if *command == "occurrence.copy" && !models.contains(&model) {
            models.push(model);
        }
    }
    models
}

/// The commands that make the definitions of `wanted` that the design does not have yet, at
/// the root, ending with the root active. `tree` is `component.list`'s reply.
pub fn definitions(tree: &Value, wanted: &[String]) -> Commands {
    let all = tree["components"].as_array().into_iter().flatten();
    let names: Vec<&str> = all.filter_map(|c| c["name"].as_str()).collect();
    let missing: Vec<&String> = (wanted.iter())
        .filter(|m| !names.contains(&m.as_str()))
        .collect();
    if missing.is_empty() {
        return vec![];
    }
    let new = !names.contains(&HOME);
    let create = |name: &str| ("component.create", json!({"name": name, "grounded": false}));
    let activate = |name: &str| ("component.activate", json!({"component": name}));
    // Where `HOME` stands, absolutely.
    let at = |to: [f64; 12]| {
        (
            "occurrence.move",
            json!({"occurrence": format!("{HOME}:1"), "to_matrix": to}),
        )
    };
    let mut commands = vec![activate("root")];
    if new {
        commands.push(create(HOME));
    } else {
        // In a plugin run a body made in a moved component lands shifted by where it stands
        // (as a hole does, see `inside` in lib.rs): so `HOME` goes back to the origin while
        // a definition is added, and under the floor again after.
        commands.push(at(ORIGIN));
        commands.push(activate(HOME));
    }
    for model in missing {
        commands.push(create(model));
        commands.extend(shape(model));
        commands.push(activate(HOME));
    }
    commands.push(activate("root"));
    commands.push(at(HIDDEN));
    commands
}

/// A model's body, about its own origin, its -Y into the board it sits on and its -X towards
/// what it hangs by (`copy`). A hinge: the cup, its centre on the door's back face, and an arm
/// from it back over the plate, lying on the plate's outer face (`ARM_REACH`, cranked as its
/// name says). A plate: a block on the side's face over its two screw holes. A slide member: a
/// bar (`MEMBERS`) centred on X, from the box's front back along +Y, as long as its name says.
fn shape(model: &str) -> Commands {
    let block = |corner: [&str; 3], size: [&str; 3]| {
        json!({
            "corner": corner, "length": size[0], "width": size[1], "height": size[2],
            "body_name": model, "name": model,
        })
    };
    if let Some(length) = model.strip_prefix(SLIDE_UNDER) {
        // the runner: a bar centred on X, its top 1 mm over the bottom's lower face
        let [w, h] = UNDER_SIZE;
        let corner = [mm(-w / 2.0), mm(0.0), mm(UNDER_RECESS - h)];
        let size = [mm(w), length.trim().to_string(), mm(h)];
        return vec![(
            "solid.box",
            block(
                [&corner[0], &corner[1], &corner[2]],
                [&size[0], &size[1], &size[2]],
            ),
        )];
    }
    let member = MEMBERS.iter().find(|m| model.starts_with(m.0));
    if let Some(&(name, t, height)) = member {
        let length = model.strip_prefix(name).unwrap_or_default().trim();
        let corner = [mm(-t / 2.0), mm(0.0), mm(-height / 2.0)];
        let size = [mm(t), length.to_string(), mm(height)];
        return vec![(
            "solid.box",
            block(
                [&corner[0], &corner[1], &corner[2]],
                [&size[0], &size[1], &size[2]],
            ),
        )];
    }
    let [plate_t, plate_d, plate_h] = PLATE_SIZE;
    match model {
        PLATE => vec![(
            "solid.box",
            block(
                [&mm(-plate_d / 2.0), &mm(0.0), &mm(-plate_h / 2.0)],
                [&mm(plate_d), &mm(plate_t), &mm(plate_h)],
            ),
        )],
        _ => {
            let crank: f64 = model
                .strip_prefix("HW hinge crank ")
                .map_or(0.0, |c| c.parse().unwrap_or(0.0));
            let cup = json!({
                "base": [mm(0.0), mm(0.0), mm(0.0)], "axis": [0, -1, 0],
                "diameter": "HW_cup_d", "height": "HW_cup_depth",
                "body_name": model, "name": format!("{model} cup"),
            });
            // On the plate's outer face (the model's X is away from the hinge edge), from 1 mm
            // into the cup, so the two are one body, back to the plate's end.
            let [arm_t, arm_l, arm_h] = ARM_SIZE;
            let x0 = plate_t - ARM_REACH - crank;
            let mut arm = block(
                [&mm(x0), &mm(-1.0), &mm(-arm_h / 2.0)],
                [&mm(arm_t), &mm(arm_l + 1.0), &mm(arm_h)],
            );
            arm["operation"] = json!("join");
            arm["targets"] = json!([model]);
            arm["name"] = json!(format!("{model} arm"));
            arm.as_object_mut().map(|a| a.remove("body_name"));
            vec![("solid.cylinder", cup), ("solid.box", arm)]
        }
    }
}

/// A copy of `model` in the cabinet's component `parent`, at `at` in that component's
/// coordinates (expressions, so it follows the parameters), turned so that the model's -Y
/// points along `into` and its -X along `toward` (unit axes at a right angle): for a hinge,
/// from the cup towards the hinge edge. Nothing with the models off. With `cut`, a feature
/// name, the copy also cuts the body named like `parent` (the door): `lib.rs` does that after
/// the build, as it needs the copy's name in the world.
pub fn copy(
    kit: &Kit,
    model: &str,
    parent: &str,
    at: Value,
    [into, toward]: [[i32; 3]; 2],
    cut: Option<String>,
) {
    if kit.models == "off" {
        return;
    }
    // The turn's columns: where the model's X, Y and Z go.
    let (x, y) = (toward.map(|v| -v), into.map(|v| -v));
    let z = [
        x[1] * y[2] - x[2] * y[1],
        x[2] * y[0] - x[0] * y[2],
        x[0] * y[1] - x[1] * y[0],
    ];
    let row = |r: usize| json!([x[r], y[r], z[r], at[r]]);
    let matrix = json!([row(0), row(1), row(2)]);
    let mut args = json!({"component": model, "parent": parent, "_matrix": matrix});
    if let Some(name) = cut {
        args["_cut"] = json!(name);
    }
    kit.push(("occurrence.copy", args));
}

/// `toward` for a model that only has to stand upright: square to `into`, turning its Z up.
pub fn upright(into: [i32; 3]) -> [[i32; 3]; 2] {
    [into, [into[1], -into[0], 0]]
}

/// The app commands a builder's list takes when run: a copy is moved after it is made, a cut
/// is a combine with its component active, after one `component.list`; and the shared
/// parameters and the definitions (and going back to the user's component), which the first
/// cabinet in a design makes.
pub fn cost(commands: &Commands) -> usize {
    let copies = commands.iter().filter(|c| c.0 == "occurrence.copy");
    let cuts = copies.clone().filter(|c| c.1.get("_cut").is_some()).count();
    let copies = copies.count();
    let shared = if used(commands) { PARAMETERS.len() } else { 0 };
    let definitions = match copies {
        0 => 0,
        _ => definitions(&json!({}), &wanted(commands)).len() + 2, // and back to the user's component; one more when `HOME` is there already
    };
    commands.len() + copies + 2 * cuts + usize::from(cuts > 0) + shared + definitions
}

/// A copy's body name in the world, as `solid.combine` takes it: `HW hinge (Base:1/Base
/// Door:1/HW hinge:2)`, the occurrences' names from the root. `tree` is `component.list`'s
/// reply, `occurrence` the copy's id.
pub fn world_name(tree: &Value, occurrence: &Value, body: &str) -> Option<String> {
    let components = tree["components"].as_array()?;
    let placements = || {
        let all = components.iter();
        all.flat_map(|c| c["occurrences"].as_array().into_iter().flatten())
    };
    let mut at = placements().find(|o| &o["id"] == occurrence)?;
    let mut path = Vec::new();
    loop {
        path.insert(0, at["name"].as_str()?);
        let parent = at["parent"].as_u64()?;
        let parent = components.iter().find(|c| c["id"] == parent)?;
        match parent["occurrences"].get(0) {
            Some(up) => at = up,
            None => break, // the root
        }
    }
    Some(format!("{body} ({})", path.join("/")))
}

/// Whether a builder's commands use the shared parameters.
pub fn used(commands: &Commands) -> bool {
    commands.iter().any(|c| c.1.to_string().contains("HW_"))
}

#[cfg(test)]
mod tests;

//! The hardware the cabinet builders share: shelf pins on the 32 mm system, hinge cups, and
//! the screw holes for knobs and pulls. Sizes are common-practice values.

use crate::kit::{mm, Kit};
use serde_json::{json, Value};

pub const PITCH: f64 = 32.0; // 32 mm system: shelf pin holes
pub const PIN_SETBACK: f64 = 37.0; // pin rows, from the front and from the back of the shelf
pub const PIN_D: f64 = 5.0;
pub const DOOR_GAP: f64 = 2.0; // the default reveal: around and between the fronts
pub const CUP_D: f64 = 35.0; // hinge cup
pub const CUP_DEPTH: f64 = 13.0;
pub const CUP_CENTRE: f64 = 22.5; // from the door edge: 5 mm of wood outside the cup
pub const CUP_FROM_END: f64 = 100.0; // from the door's top and bottom
pub const HANDLE_D: f64 = 5.0; // screw hole for a knob or pull
pub const PULL: f64 = 128.0; // between the two screws of a pull
pub const HANDLE_EDGE: f64 = 50.0; // on a door: from its opening edge
pub const HANDLE_END: f64 = 100.0; // and from its top or bottom to the nearest hole

/// The top row of pins, counted in pitches above the bottom panel. Pin holes sit on a 32 mm
/// grid there, two pitches clear of both ends of the clear height.
pub fn top_pin_row(clear_h: &str) -> String {
    format!("floor(({clear_h} - {}) / {})", mm(2.0 * PITCH), mm(PITCH))
}

/// A shelf's height `over` the bottom panel, moved to the nearest row of pins.
pub fn on_pin_row(over: &str, top_row: &str) -> String {
    let (pitch, r) = (mm(PITCH), mm(PIN_D / 2.0));
    format!("min(max(round(({over} - {r}) / {pitch}), 2), {top_row}) * {pitch} + {r}")
}

/// `on_pin_row` in mm, for where a point on the shelf is. `clear_h` is the clear height.
pub fn on_pin_row_mm(over: f64, clear_h: f64) -> f64 {
    let top = ((clear_h - 2.0 * PITCH) / PITCH).floor();
    ((over - PIN_D / 2.0) / PITCH).round().max(2.0).min(top) * PITCH + PIN_D / 2.0
}

/// How deep a pin hole goes into a panel it must not come through.
pub fn blind_pin(thickness: &str) -> String {
    format!("min({thickness} - {}, {})", mm(5.0), mm(12.0))
}

/// Pin holes in one panel.
pub struct PinRows<'a> {
    pub part: &'a str,
    /// The lowest hole of the first row, on the panel's face: `Kit::at` made it.
    pub position: Value,
    /// The way into the panel, and its depth.
    pub into: [i32; 3],
    pub depth: &'a str,
    /// The way to the second row, and what the two rows are `PIN_SETBACK` in from.
    pub across: [i32; 3],
    pub run: &'a str,
}

/// One hole, repeated up the panel to `top_row` and across to the second row.
pub fn pin_rows(kit: &Kit, top_row: &str, rows: PinRows) -> [(&'static str, Value); 2] {
    let (name, part) = (&kit.name, rows.part);
    let feature = format!("{name} {part} pins");
    [
        (
            "solid.cylinder",
            json!({
                "base": rows.position, "axis": rows.into,
                "diameter": mm(PIN_D), "height": rows.depth, "operation": "cut",
                "targets": [format!("{name} {part}")], "name": feature,
            }),
        ),
        (
            "solid.pattern.rectangular",
            json!({
                "features": [feature], "name": format!("{name} {part} pin rows"),
                "dir1": [0, 0, 1], "count1": format!("{top_row} - 1"), "spacing1": mm(PITCH),
                "dir2": rows.across, "count2": 2,
                "spacing2": format!("{} - {}", rows.run, mm(2.0 * PIN_SETBACK)),
            }),
        ),
    ]
}

// Every hole is a cylinder cut out of the one board it is for, not `solid.hole`: that keeps
// the face it was picked on, and when the history is built again in a design with many
// cabinets (after one is deleted, as Edit does) the face is not found and the hole fails.
/// A hole of diameter `d` in a board of the cabinet, `into` it from `position`.
pub fn hole(
    kit: &Kit,
    part: &str,
    position: Value,
    into: [i32; 3],
    d: &str,
    depth: &str,
    what: &str,
) {
    let name = &kit.name;
    kit.push((
        "solid.cylinder",
        json!({
            "base": position, "axis": into,
            "diameter": d, "height": depth, "operation": "cut",
            "targets": [format!("{name} {part}")],
            "name": format!("{name} {part} {what}"),
        }),
    ));
}

/// Where the hinge cups go on a door from `z0` up to `z1`, `door_h` mm high: one near each
/// end, and a third in the middle of a door over 1000 mm.
pub fn cup_heights(z0: &str, z1: &str, door_h: f64) -> Vec<String> {
    let mut cups = vec![
        format!("{z0} + {}", mm(CUP_FROM_END)),
        format!("{z1} - {}", mm(CUP_FROM_END)),
    ];
    if door_h > 1000.0 {
        cups.push(format!("({z0} + {z1}) / 2"));
    }
    cups
}

/// Where the screws of a knob (one) or a pull (two) sit, from the handle's middle.
pub fn screw_offsets(handles: &str) -> &'static [f64] {
    match handles {
        "none" => &[],
        "knob" => &[0.0],
        _ => &[-PULL / 2.0, PULL / 2.0],
    }
}

/// The height of each screw of a handle whose middle is at `handle_z`.
pub fn screw_heights(handles: &str, handle_z: &str) -> Vec<String> {
    let screws = screw_offsets(handles).iter().map(|&offset| {
        if offset < 0.0 {
            format!("{handle_z} - {}", mm(-offset))
        } else if offset > 0.0 {
            format!("{handle_z} + {}", mm(offset))
        } else {
            handle_z.to_string()
        }
    });
    screws.collect()
}

/// The `handle_position` field: where on a door its handle sits.
pub fn handle_position(p: &Value) -> Result<&str, String> {
    let at = p.get("handle_position").and_then(Value::as_str);
    match at.unwrap_or("auto") {
        at @ ("auto" | "top" | "middle" | "bottom") => Ok(at),
        _ => Err("handle_position must be auto, top, middle or bottom".into()),
    }
}

pub const KICK_RECESS: f64 = 50.0; // `plinth_setback`'s default: the board or notch, back from the front
pub const LEG_D: f64 = 40.0; // an adjustable leg
pub const LEG_INSET: f64 = 50.0; // a leg's middle, in from the carcass's outside edges
pub const CASTER_MIN: f64 = 50.0; // the lowest plinth a caster fits under
pub const CASTER_PLATE: f64 = 80.0; // a caster's square mounting plate
pub const CASTER_SCREWS: f64 = 60.0; // between the plate's screw holes, both ways
pub const CASTER_SCREW_D: f64 = 4.0; // a pilot hole for one of them

/// The `plinth_type` field: `board` (one set back, the default), `notch` (the sides notched
/// at the front, no board: for one long board across a run), `legs` (the carcass on legs) or
/// `casters` (on casters: box cabinets only, `corner::input` refuses it).
pub fn plinth_type(p: &Value) -> Result<&str, String> {
    match p
        .get("plinth_type")
        .and_then(Value::as_str)
        .unwrap_or("board")
    {
        kind @ ("board" | "notch" | "legs" | "casters") => Ok(kind),
        _ => Err("plinth_type must be board, notch, legs or casters".into()),
    }
}

/// An adjustable leg, `<Name> Leg <n>`: a cylinder from the floor up to the bottom panel,
/// `height` high, under `at` (`Kit::at`, at the floor). A body of its own, not a board: the
/// cut list counts it as hardware.
pub fn leg(kit: &Kit, n: usize, at: Value, height: &str) {
    let body = format!("{} Leg {n}", kit.name);
    kit.push((
        "solid.cylinder",
        json!({
            "base": at, "axis": [0, 0, 1], "diameter": mm(LEG_D), "height": height,
            "name": body, "body_name": body,
        }),
    ));
}

/// A locking caster, `<Name> Caster <n>`, one body `height` high under `(x, y)` (from the
/// carcass's front left corner, at the floor): a square plate under the bottom panel, a post
/// and a wheel on the floor. The plate's four screws get pilot holes in the bottom panel, from
/// its underside: one cut and a 2 x 2 pattern, at most half the panel deep so they stay clear
/// of a back groove in its top.
pub fn caster(kit: &Kit, n: usize, x: &str, y: &str, height: &str, thickness: &str) {
    let name = &kit.name;
    let body = format!("{name} Caster {n}");
    let (plate, plate_t, wheel_w) = (mm(CASTER_PLATE), mm(5.0), mm(25.0));
    let half = |v: f64| mm(v / 2.0);
    let plate_at = kit.at(
        &format!("{x} - {}", half(CASTER_PLATE)),
        &format!("{y} - {}", half(CASTER_PLATE)),
        &format!("{height} - {plate_t}"),
    );
    // A wheel of at most 75 mm on the floor, so it stays under the carcass on a high plinth,
    // and a post from its middle 2 mm into the 5 mm plate: one solid.
    let wheel_d = format!("min({}, {height} - {})", mm(75.0), mm(2.0));
    let wheel_at = kit.at(&format!("{x} - {wheel_w}/2"), y, &format!("{wheel_d}/2"));
    let post_h = format!("{height} - {} - {wheel_d}/2", mm(3.0));
    let screw = |v: &str| format!("{v} - {}", half(CASTER_SCREWS));
    let feature = format!("{name} Bottom caster {n}");
    kit.push((
        "solid.box",
        json!({
            "length": plate, "width": plate, "height": plate_t, "corner": plate_at,
            "name": body, "body_name": body,
        }),
    ));
    kit.push((
        "solid.cylinder",
        json!({
            "base": wheel_at, "axis": [1, 0, 0], "diameter": wheel_d, "height": wheel_w,
            "operation": "join", "targets": [body], "name": format!("{body} wheel"),
        }),
    ));
    kit.push((
        "solid.cylinder",
        json!({
            "base": kit.at(x, y, &format!("{wheel_d}/2")), "axis": [0, 0, 1],
            "diameter": mm(20.0), "height": post_h,
            "operation": "join", "targets": [body], "name": format!("{body} post"),
        }),
    ));
    kit.push((
        "solid.cylinder",
        json!({
            "base": kit.at(&screw(x), &screw(y), height), "axis": [0, 0, 1],
            "diameter": mm(CASTER_SCREW_D),
            "height": format!("min({}, {thickness}/2 - {})", mm(10.0), mm(1.0)),
            "operation": "cut", "targets": [format!("{name} Bottom")], "name": feature,
        }),
    ));
    kit.push((
        "solid.pattern.rectangular",
        json!({
            "features": [feature], "name": format!("{feature} screws"),
            "dir1": [1, 0, 0], "count1": 2, "spacing1": mm(CASTER_SCREWS),
            "dir2": [0, 1, 0], "count2": 2, "spacing2": mm(CASTER_SCREWS),
        }),
    ));
}

pub const NAILER_H: f64 = 90.0; // ponytail: a hanging rail is always this high, no field for it

/// The `wall_mount` field and its board's thickness and height in mm (0 without one): none,
/// cleat (a French cleat, sized by its fields), rail (a hanging rail, a panel thick) or steel
/// (a bought rail and hangers, no board). `between` is the height between bottom and top.
pub fn wall_mount<'a>(
    kit: &Kit,
    p: &'a Value,
    t: f64,
    between: f64,
) -> Result<(&'a str, f64, f64), String> {
    let mount = match p
        .get("wall_mount")
        .and_then(Value::as_str)
        .unwrap_or("none")
    {
        m @ ("none" | "cleat" | "rail" | "steel") => m,
        _ => return Err("wall_mount must be none, cleat, rail or steel".into()),
    };
    let (mount_t, mount_h) = match mount {
        "cleat" => {
            let ct = kit.size(p, "cleat_thickness", 18.0)?;
            let ch = kit.size(p, "cleat_height", 90.0)?;
            crate::kit::check(
                (6.0..=50.0).contains(&ct),
                "cleat thickness must be 6 to 50 mm",
            )?;
            crate::kit::check(ch > ct, "the cleat must be taller than it is thick")?;
            // the wall cleat hangs below the cabinet's, overlapping it by the bevel
            crate::kit::check(
                2.0 * ch - ct <= between,
                "the cleats do not fit: twice the cleat height less its thickness must fit between the bottom and the top",
            )?;
            (ct, ch)
        }
        "rail" => {
            crate::kit::check(NAILER_H <= between, "too low for a hanging rail")?;
            (t, NAILER_H)
        }
        _ => (0.0, 0.0),
    };
    Ok((mount, mount_t, mount_h))
}

/// The mount board's thickness and height as expressions (a cleat's are parameters), or
/// `None` for no board. `et` is the panel thickness.
pub fn mount_sizes(
    kit: &Kit,
    mount: &str,
    mount_t: f64,
    mount_h: f64,
    et: &str,
) -> Option<[String; 2]> {
    match mount {
        "cleat" => Some([
            kit.parameter("cleat_thickness", mount_t, "French cleat thickness"),
            kit.parameter("cleat_height", mount_h, "French cleat height"),
        ]),
        "rail" => Some([et.to_string(), mm(NAILER_H)]),
        _ => None,
    }
}

// ponytail: steel is not modelled (hanger models and their holes come with #70); the cut
// list buys a rail to this length and two hangers. Kept at 0 once an edit stops using it.
/// A steel wall rail, the parameter `<Prefix>_<key>`, `length` long (mm and expression).
pub fn steel_rail(kit: &Kit, key: &str, steel: bool, length: f64, expression: &str) {
    if steel || kit.has(key) {
        let (value, length) = match steel {
            true => (length, expression.to_string()),
            false => (0.0, mm(0.0)),
        };
        kit.parameter_as(key, value, "steel wall rail, bought to length", length);
    }
}

/// The board a cabinet hangs by, along one wall under its top: a hanging rail (`mount` rail),
/// or a French cleat and the wall's cleat nested below it (cleat). `panel` makes a board from
/// its corner and size; `board` is the cabinet's (corner and size, its height the mount's),
/// `tm` the mount's thickness, `suffix` follows each part's name. `edges` are the box edges to
/// bevel by `tm`: the cabinet cleat's under its inner face, the wall cleat's on top at the
/// wall. Box edges by index, so they follow a resize.
pub fn mount_board(
    kit: &Kit,
    mount: &str,
    tm: &str,
    panel: impl Fn(&str, [&str; 3], [&str; 3]),
    [at, size]: [[&str; 3]; 2],
    suffix: &str,
    edges: [u32; 2],
) {
    if mount == "rail" {
        return panel(&format!("Hanging rail{suffix}"), at, size);
    }
    let wall_z = format!("{} + {tm} - {}", at[2], size[2]);
    let parts = [
        ("Cleat", at[2], edges[0]),
        ("Wall cleat", &wall_z, edges[1]),
    ];
    for (part, z, edge) in parts {
        let part = format!("{part}{suffix}");
        panel(&part, [at[0], at[1], z], size);
        let body = format!("{} {part}", kit.name);
        kit.push((
            "solid.chamfer",
            json!({
                "edges": [{"body": body, "index": edge}], "distance": tm,
                "body": body, "name": format!("{body} bevel"),
            }),
        ));
    }
}

/// Whether a body is one of the legs `leg` makes.
pub fn is_leg(body: &str) -> bool {
    numbered(body, " Leg ")
}

/// Whether a body is one of the casters `caster` makes.
pub fn is_caster(body: &str) -> bool {
    numbered(body, " Caster ")
}

/// A leg or a caster: hardware the carcass stands on, not a board.
pub fn is_foot(body: &str) -> bool {
    is_leg(body) || is_caster(body)
}

fn numbered(body: &str, part: &str) -> bool {
    body.rsplit_once(part)
        .is_some_and(|(_, n)| n.parse::<u32>().is_ok())
}

// ponytail: on `auto` a door's handle is placed by a guess from the cabinet: mid-height on a
// tall door, near the top on a floor unit, else near the bottom as on a wall unit.
/// A floor unit has a plinth or is deep; a wall unit is shallow and has none.
pub fn floor_unit(plinth: f64, depth: f64) -> bool {
    plinth > 0.0 || depth >= 450.0
}

/// The middle of the handle on a door from `z0` up to `z1`, at `at`: top, middle, bottom or
/// auto. `front_h` is the cabinet's height above its plinth, in mm.
pub fn handle_height(
    handles: &str,
    at: &str,
    z0: &str,
    z1: &str,
    front_h: f64,
    floor: bool,
) -> String {
    let half = if handles == "pull" { PULL / 2.0 } else { 0.0 };
    let end = mm(HANDLE_END + half);
    let at = match at {
        "auto" if front_h > 1200.0 => "middle",
        "auto" if floor => "top",
        "auto" => "bottom",
        at => at,
    };
    match at {
        "middle" => format!("({z0} + {z1}) / 2"),
        "top" => format!("{z1} - {end}"),
        _ => format!("{z0} + {end}"),
    }
}

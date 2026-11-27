//! The hardware the cabinet builders share: shelf pins on the 32 mm system, hinge cups, and
//! the screw holes for knobs and pulls. Sizes are common-practice values.

use crate::kit::{mm, Kit};
use crate::Commands;
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
pub const GROOVE_D: f64 = 20.0; // a finger groove: a half round, 20 mm high and 10 deep
pub const GROOVE_EDGE: f64 = 25.0; // its middle, from the front's edge
pub const GRIP_LAND: f64 = 6.0; // a grip edge's 45-degree bevel leaves this much of the edge
pub const TAB_OUT: f64 = 20.0; // a drawer bottom's tab (#181): how far past the front it sticks out,
pub const TAB_R: f64 = 10.0; // and the radius of its two front corners

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

/// The pin rows of all a cabinet's panels (`pin_rows`' pairs, in order) with one pattern for the
/// panels whose rows match (same count, pitch and rows across): a feature less per panel, and
/// a pattern feature costs time. The holes come first, then the patterns, named `<Cabinet> pin
/// rows` (` 2`, ... for a second set), or the panel's own when it matches no other.
pub fn shared_pin_rows(kit: &Kit, drill: Commands) -> Commands {
    let key = |args: &Value| {
        let mut key = args.clone();
        if let Some(k) = key.as_object_mut() {
            k.remove("features");
            k.remove("name");
        }
        key
    };
    let (mut holes, mut patterns): (Commands, Vec<(Value, Value)>) = (Vec::new(), Vec::new());
    for (command, args) in drill {
        if command != "solid.pattern.rectangular" {
            holes.push((command, args));
            continue;
        }
        let features = args["features"].as_array().cloned().unwrap_or_default();
        match patterns.iter_mut().find(|p| p.0 == key(&args)) {
            Some((_, merged)) => {
                if let Some(all) = merged["features"].as_array_mut() {
                    all.extend(features);
                }
            }
            None => patterns.push((key(&args), args)),
        }
    }
    let mut shared = 0;
    for (_, mut args) in patterns {
        if args["features"].as_array().is_some_and(|f| f.len() > 1) {
            shared += 1;
            args["name"] = json!(match shared {
                1 => format!("{} pin rows", kit.name),
                n => format!("{} pin rows {n}", kit.name),
            });
        }
        holes.push(("solid.pattern.rectangular", args));
    }
    holes
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
            "targets": kit.targets(part),
            "name": format!("{name} {part} {what}"),
        }),
    ));
}

/// Where the hinge cups go on a door from `z0` up to `z1`, `door_h` mm high: one near each
/// end (the first two), and the rest evenly between them. Two hinges up to 900 mm, three up
/// to 1600, four up to 2000, five above (`HINGES`).
pub fn cup_heights(z0: &str, z1: &str, door_h: f64) -> Vec<String> {
    let end = mm(CUP_FROM_END);
    let mut cups = vec![format!("{z0} + {end}"), format!("{z1} - {end}")];
    let n = HINGES.iter().filter(|&&up_to| door_h > up_to).count() + 2;
    for k in 1..n - 1 {
        let gap = format!("({z1} - ({z0}) - 2*{end})");
        cups.push(format!("{z0} + {end} + {gap} * {k} / {}", n - 1));
    }
    cups
}

/// The door heights in mm past which a door takes one hinge more (a hinge maker's chart for
/// 18 to 19 mm doors).
const HINGES: [f64; 3] = [900.0, 1600.0, 2000.0];

/// Where the screws of a knob (one) or a pull (two) sit, from the handle's middle.
pub fn screw_offsets(handles: &str) -> &'static [f64] {
    match handles {
        "none" | "grip" | "groove" | "tab" => &[],
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

/// Whether a handle-less front of `t` mm keeps wood behind its grip: a grip edge's bevel
/// leaves `GRIP_LAND`, and a finger groove must not come through.
pub fn profile_fits(handles: &str, t: f64) -> Result<(), String> {
    match handles {
        "grip" if t <= GRIP_LAND => Err(format!("a grip edge needs fronts over {GRIP_LAND} mm")),
        "groove" if t <= GROOVE_D / 2.0 => Err(format!(
            "a finger groove needs fronts over {} mm",
            GROOVE_D / 2.0
        )),
        "tab" if t <= GROOVE_D / 2.0 => Err(format!(
            "a bottom tab's scoop needs fronts over {} mm",
            GROOVE_D / 2.0
        )),
        _ => Ok(()),
    }
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
/// `casters` (on casters, under a `plinth` of `CASTER_MIN` or more; 0 is no plinth).
pub fn plinth_type(p: &Value, plinth: f64) -> Result<&str, String> {
    let kind = match p
        .get("plinth_type")
        .and_then(Value::as_str)
        .unwrap_or("board")
    {
        kind @ ("board" | "notch" | "legs" | "casters") => kind,
        _ => return Err("plinth_type must be board, notch, legs or casters".into()),
    };
    crate::kit::check(
        plinth == 0.0 || kind != "casters" || plinth >= CASTER_MIN,
        "casters need a plinth of at least 50 mm",
    )?;
    Ok(kind)
}

/// A leg or (`kind` casters) a caster, number `n`, under `(x, y)` from the carcass's front
/// left corner at the floor, `height` high; a caster's screws go into the board `bottom`.
pub fn foot(
    kit: &Kit,
    kind: &str,
    n: usize,
    [x, y]: [&str; 2],
    height: &str,
    [t, bottom]: [&str; 2],
) {
    match kind {
        "legs" => leg(kit, n, kit.at(x, y, &mm(0.0)), height),
        _ => caster(kit, n, x, y, height, t, bottom),
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
/// of a back groove in its top. `bottom` is that panel's part name.
pub fn caster(kit: &Kit, n: usize, x: &str, y: &str, height: &str, thickness: &str, bottom: &str) {
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
    let feature = format!("{name} {bottom} caster {n}");
    kit.push((
        "solid.box",
        json!({
            "length": plate, "width": plate, "height": plate_t, "corner": plate_at,
            "name": body, "body_name": body,
        }),
    ));
    // The post first: joined before it, the wheel would not touch the plate yet.
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
            "base": wheel_at, "axis": [1, 0, 0], "diameter": wheel_d, "height": wheel_w,
            "operation": "join", "targets": [body], "name": format!("{body} wheel"),
        }),
    ));
    kit.push((
        "solid.cylinder",
        json!({
            "base": kit.at(&screw(x), &screw(y), height), "axis": [0, 0, 1],
            "diameter": mm(CASTER_SCREW_D),
            "height": format!("min({}, {thickness}/2 - {})", mm(10.0), mm(1.0)),
            "operation": "cut", "targets": [format!("{name} {bottom}")], "name": feature,
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

/// `handle_position` with auto settled: top, middle or bottom.
fn place(at: &str, front_h: f64, floor: bool) -> &str {
    match at {
        "auto" if front_h > 1200.0 => "middle",
        "auto" if floor => "top",
        "auto" => "bottom",
        at => at,
    }
}

/// Grooved fronts (`front_style` vertical or horizontal, #182): `shape` square or round,
/// `spacing` and `depth` in mm (the design-wide settings `groove_spacing`, `groove_depth`).
#[derive(Clone, Copy)]
pub struct Grooves<'a> {
    pub style: &'a str,
    pub shape: &'a str,
    pub mm: [f64; 2],
}

/// The fields `front_style`, `groove_shape`, `groove_spacing` and `groove_depth` (the sizes as
/// `size` reads them, in mm): `None` for flat fronts. A groove stays clear of the hinge cups
/// behind it (`CUP_DEPTH`) on a cabinet with doors, and of half the front without.
pub fn grooves<'a>(
    p: &'a Value,
    size: impl Fn(&str, f64) -> Result<f64, String>,
    t: f64,
    doors: bool,
) -> Result<Option<Grooves<'a>>, String> {
    let style = p.get("front_style").and_then(Value::as_str);
    let shape = p.get("groove_shape").and_then(Value::as_str);
    let (style, shape) = match (style.unwrap_or("flat"), shape.unwrap_or("square")) {
        ("flat" | "shaker" | "glass", _) => return Ok(None),
        (s @ ("vertical" | "horizontal"), g @ ("square" | "round")) => (s, g),
        ("vertical" | "horizontal", _) => return Err("groove_shape must be square or round".into()),
        _ => return Err("front_style must be flat, vertical, horizontal, shaker or glass".into()),
    };
    let [spacing, depth] = [size("groove_spacing", 50.0)?, size("groove_depth", 3.0)?];
    let most = if doors { t - CUP_DEPTH - 1.0 } else { t / 2.0 };
    if !(0.5..=most).contains(&depth) {
        return Err(format!(
            "the front grooves' depth must be 0.5 to {most} mm in a {t} mm front"
        ));
    }
    if spacing < 4.0 * depth + 4.0 {
        return Err("the front grooves are too close for their depth".into());
    }
    Ok(Some(Grooves {
        style,
        shape,
        mm: [spacing, depth],
    }))
}

/// Shaker fronts (`front_style` shaker, #183): the frame's stiles and rails `frame_width` wide
/// round a `panel_thickness` panel, in mm (design-wide settings). `None` for other styles.
/// The smallest door must leave room for its panel; a stile takes the hinge cups and the handle.
pub fn frame(
    p: &Value,
    size: impl Fn(&str, f64) -> Result<f64, String>,
    t: f64,
    smallest_door: Option<f64>,
) -> Result<Option<[f64; 2]>, String> {
    if !matches!(
        p.get("front_style").and_then(Value::as_str),
        Some("shaker" | "glass")
    ) {
        return Ok(None);
    }
    let [fw, pt] = [size("frame_width", 60.0)?, size("panel_thickness", 6.0)?];
    if !(55.0..=150.0).contains(&fw) {
        return Err("a shaker frame must be 55 to 150 mm wide".into());
    }
    if !(3.0..=t - 6.0).contains(&pt) {
        return Err(format!("a shaker panel must be 3 to {} mm thick", t - 6.0));
    }
    if smallest_door.is_some_and(|d| d < framed(fw)) {
        return Err(format!(
            "doors are too small for a {fw} mm shaker frame: {} mm or more",
            framed(fw)
        ));
    }
    let handles = p.get("handles").and_then(Value::as_str);
    if matches!(handles, Some("grip" | "groove")) {
        return Err("a grip edge or finger groove needs flat fronts, not shaker or glass".into());
    }
    Ok(Some([fw, pt]))
}

/// The least width and height a front takes a frame of `fw` mm in: a panel of 60 mm or more.
pub fn framed(fw: f64) -> f64 {
    2.0 * fw + 60.0
}

/// The frame's width and panel thickness as the cabinet's parameters, made once a build.
pub fn frame_sizes(kit: &Kit, mm: [f64; 2]) -> [String; 2] {
    [
        kit.parameter("frame_width", mm[0], "shaker frame width"),
        kit.parameter("panel_thickness", mm[1], "shaker panel thickness"),
    ]
}

/// How far a door's handle is from its opening edge: `HANDLE_EDGE`, or the middle of a shaker
/// door's stile.
pub fn handle_edge(frame: Option<&[String; 2]>) -> String {
    match frame {
        Some([fw, _]) => format!("{fw}/2"),
        None => mm(HANDLE_EDGE),
    }
}

/// A glass door's pane (`front_style` glass): this thick, in the frame's groove as a panel is.
pub const GLASS_T: f64 = 4.0;

/// Whether a body is a glass door's pane (`front` with `glass`): bought, not a board.
pub fn is_glass(body: &str) -> bool {
    body.ends_with(" glass")
}

/// How deep the panel sits in the frame's groove, each side.
pub const PANEL_GROOVE: f64 = 8.0;

/// A front as one board, or with `frame` (its width and panel thickness parameters) a shaker
/// front: `<part>` is the stile at the low end of `along` (`main_low`) or at the high one, the
/// other is `<part> stile`, between them `<part> rail bottom` and `rail top`, and `<part> panel`
/// in the middle, centred in the front's thickness and `PANEL_GROOVE` into a groove all round
/// (one box cut of the panel's size through the four frame boards). The front as for `grip`.
/// Every later hole into the part cuts all five (`Kit::targets`).
// ponytail: the rails butt between the stiles; a stub tenon or dowels are not cut
#[allow(clippy::too_many_arguments)]
pub fn front(
    kit: &Kit,
    part: &str,
    frame: Option<&[String; 2]>,
    at: impl Fn(&str, &str, &str) -> Value,
    lo: [&str; 3],
    size: [&str; 3],
    along: usize,
    main_low: bool,
    glass: bool,
) {
    let Some([fw, pt]) = frame else {
        return kit.panel_at(part, at(lo[0], lo[1], lo[2]), size, None);
    };
    let across = 1 - along;
    // a box from (along, across, up) `from`, of `extent`
    let place = |from: [String; 3], extent: [String; 3]| {
        let [a, c, z] = from;
        let [sa, sc, sz] = extent;
        let mut p = [String::new(), String::new(), z];
        let mut s = [String::new(), String::new(), sz];
        (p[along], p[across], s[along], s[across]) = (a, c, sa, sc);
        (at(&p[0], &p[1], &p[2]), s)
    };
    let (a0, c0, z0) = (lo[along], lo[across], lo[2]);
    let (la, lc, lz) = (size[along], size[across], size[2]);
    let far = format!("{a0} + {la} - {fw}");
    let (main, other) = match main_low {
        true => (a0.to_string(), far),
        false => (far, a0.to_string()),
    };
    let inner = format!("{la} - 2*{fw}");
    let rail = format!("{a0} + {fw}");
    let boards = [
        (
            part.to_string(),
            main,
            z0.to_string(),
            fw.clone(),
            lz.to_string(),
        ),
        (
            format!("{part} stile"),
            other,
            z0.to_string(),
            fw.clone(),
            lz.to_string(),
        ),
        (
            format!("{part} rail bottom"),
            rail.clone(),
            z0.to_string(),
            inner.clone(),
            fw.clone(),
        ),
        (
            format!("{part} rail top"),
            rail,
            format!("{z0} + {lz} - {fw}"),
            inner,
            fw.clone(),
        ),
    ];
    let mut all = Vec::new();
    for (name, a, z, sa, sz) in boards {
        let (corner, s) = place([a, c0.to_string(), z], [sa, lc.to_string(), sz]);
        kit.panel_at(&name, corner, [&s[0], &s[1], &s[2]], None);
        all.push(format!("{} {name}", kit.name));
    }
    let g = mm(PANEL_GROOVE);
    // a glass door's pane in the panel's place, `GLASS_T` thick; holes leave it out
    let (pane, pt) = match glass {
        true => ("glass", mm(GLASS_T)),
        false => ("panel", pt.clone()),
    };
    let (corner, s) = place(
        [
            format!("{a0} + {fw} - {g}"),
            format!("{c0} + ({lc} - {pt})/2"),
            format!("{z0} + {fw} - {g}"),
        ],
        [
            format!("{la} - 2*{fw} + 2*{g}"),
            pt,
            format!("{lz} - 2*{fw} + 2*{g}"),
        ],
    );
    let size = [s[0].as_str(), &s[1], &s[2]];
    kit.panel_at(&format!("{part} groove"), corner.clone(), size, Some(&all));
    kit.panel_at(&format!("{part} {pane}"), corner, size, None);
    if !glass {
        all.push(format!("{} {part} panel", kit.name));
    }
    kit.pieces(part, all);
}

/// The grooves' spacing and depth as the cabinet's parameters, made once a build.
pub fn groove_sizes(kit: &Kit, g: &Grooves) -> [String; 2] {
    [
        kit.parameter("groove_spacing", g.mm[0], "front groove spacing"),
        kit.parameter("groove_depth", g.mm[1], "front groove depth"),
    ]
}

/// Grooves in a front's face, `sizes` apart about (the spacing evened out so the strips between
/// are alike) and as deep, `2*depth` wide: square (a box cut) or round (a half-round cylinder
/// cut), each run the front's whole length and a millimetre past it. One cut and one pattern,
/// its count an expression, so they follow a resize. The front as for `grip`.
#[allow(clippy::too_many_arguments)]
pub fn front_grooves(
    kit: &Kit,
    part: &str,
    g: &Grooves,
    [spacing, depth]: &[String; 2],
    at: impl Fn(&str, &str, &str) -> Value,
    lo: [&str; 3],
    size: [&str; 3],
    along: usize,
    back: i32,
) {
    let across = 1 - along;
    // the grooves run along `run` and repeat along `step`
    let (run, step) = match g.style {
        "vertical" => (2, along),
        _ => (along, 2),
    };
    let strips = format!("max(round(({}) / {spacing}), 2)", size[step]);
    let pitch = format!("({}) / {strips}", size[step]);
    let first = format!("{} + {pitch}", lo[step]);
    let face = match back {
        1 => lo[across].to_string(),
        _ => format!("{} + {}", lo[across], size[across]),
    };
    let mut axis = [0, 0, 0];
    axis[run] = 1;
    let mut p = [String::new(), String::new(), String::new()];
    let name = format!("{} {part} front groove", kit.name);
    let length = format!("{} + 2 mm", size[run]);
    p[run] = format!("{} - 1 mm", lo[run]);
    match g.shape {
        "round" => {
            p[step] = first;
            p[across] = face;
            let d = format!("2*{depth}");
            hole(
                kit,
                part,
                at(&p[0], &p[1], &p[2]),
                axis,
                &d,
                &length,
                "front groove",
            );
        }
        _ => {
            // from a millimetre in front of the face to `depth` behind it
            p[step] = format!("{first} - {depth}");
            p[across] = match back {
                1 => format!("{face} - 1 mm"),
                _ => format!("{face} - {depth}"),
            };
            let mut dims = [String::new(), String::new(), String::new()];
            dims[run] = length;
            dims[step] = format!("2*{depth}");
            dims[across] = format!("{depth} + 1 mm");
            kit.push((
                "solid.box",
                json!({
                    "corner": at(&p[0], &p[1], &p[2]), "length": dims[0], "width": dims[1],
                    "height": dims[2], "operation": "cut",
                    "targets": [format!("{} {part}", kit.name)], "name": name,
                }),
            ));
        }
    }
    let mut dir = [0, 0, 0];
    dir[step] = 1;
    kit.push((
        "solid.pattern.rectangular",
        json!({
            "features": [name], "dir1": dir, "count1": format!("{strips} - 1"),
            "spacing1": pitch, "name": format!("{name}s"),
        }),
    ));
}

/// `grip_gap` (#230): with a grip edge, how far a front stops short of its grip edge, in mm,
/// so a finger fits in the gap behind the bevel; `None` for other handles. A grip edge goes on
/// the top or the bottom edge, never the middle, and leaves the front `MIN_GRIPPED` high.
pub fn grip_gap(
    handles: &str,
    at: &str,
    size: impl Fn(&str, f64) -> Result<f64, String>,
    lowest_front: f64,
) -> Result<Option<f64>, String> {
    if handles != "grip" {
        return Ok(None);
    }
    if at == "middle" {
        return Err(
            "a grip edge goes on the top or the bottom edge (Handle position), not the middle"
                .into(),
        );
    }
    let gap = size("grip_gap", GRIP_GAP)?;
    if !(5.0..=50.0).contains(&gap) {
        return Err("the grip gap must be 5 to 50 mm".into());
    }
    if lowest_front - gap < MIN_GRIPPED {
        return Err(format!(
            "the fronts are too low for a {gap} mm grip gap: {MIN_GRIPPED} mm must be left"
        ));
    }
    Ok(Some(gap))
}

pub const GRIP_GAP: f64 = 20.0; // `grip_gap`'s default
const MIN_GRIPPED: f64 = 80.0; // what a grip gap must leave of a front

/// A front from `z`, `h` high, with a grip edge `gap` (the parameter) shorter at its grip edge
/// (`top`, else the bottom); as it is without one.
pub fn gripped(gap: Option<&str>, z: &str, h: &str, top: bool) -> (String, String) {
    match (gap, top) {
        (None, _) => (z.to_string(), h.to_string()),
        (Some(g), true) => (z.to_string(), format!("({h} - {g})")),
        (Some(g), false) => (format!("{z} + {g}"), format!("({h} - {g})")),
    }
}

/// Whether a door's grip edge or groove (`handles` grip or groove) is along its top edge, else
/// its bottom: auto by the kind of unit (also on a tall door), and middle is the top.
pub fn grip_on_top(at: &str, floor: bool) -> bool {
    match at {
        "auto" => floor,
        at => at != "bottom",
    }
}

/// A handle-less front's grip along its top (`top`) or bottom edge, for `handles` grip (a
/// 45-degree bevel along the back of the edge: a box turned on the edge and cut away) or groove
/// (a half-round groove in the front face, a cylinder cut); nothing for other handles. The
/// front is the box from `lo` to `lo + size` in the builder's coordinates, `along` its
/// horizontal axis (0 or 1) and `back` the way (1 or -1) across it to its back face; `at` turns
/// a point into the command's. Both cuts reach a millimetre past the ends.
#[allow(clippy::too_many_arguments)]
pub fn grip(
    kit: &Kit,
    part: &str,
    handles: &str,
    at: impl Fn(&str, &str, &str) -> Value,
    lo: [&str; 3],
    size: [&str; 3],
    along: usize,
    back: i32,
    top: bool,
) {
    let across = 1 - along;
    let face = |back_face: bool| match (back == 1) == back_face {
        true => format!("{} + {}", lo[across], size[across]),
        false => lo[across].to_string(),
    };
    let edge = match top {
        true => format!("{} + {}", lo[2], size[2]),
        false => lo[2].to_string(),
    };
    let mut axis = [0, 0, 0];
    axis[along] = 1;
    let point = |a: String, c: String, z: String| {
        let mut p = [String::new(), String::new(), z];
        (p[along], p[across]) = (a, c);
        at(&p[0], &p[1], &p[2])
    };
    let body = format!("{} {part}", kit.name);
    match handles {
        "groove" => {
            let z = match top {
                true => format!("{edge} - {}", mm(GROOVE_EDGE)),
                false => format!("{edge} + {}", mm(GROOVE_EDGE)),
            };
            let base = point(format!("{} - 1 mm", lo[along]), face(false), z);
            let length = format!("{} + 2 mm", size[along]);
            hole(kit, part, base, axis, &mm(GROOVE_D), &length, "groove");
        }
        "grip" => {
            // A square on the back edge, turned 45 degrees: what it takes of the front is the
            // bevel, `t - GRIP_LAND` along both faces.
            let centre = point(
                format!("{} + ({})/2", lo[along], size[along]),
                face(true),
                edge,
            );
            let side = format!("sqrt(2)*({} - {})", size[across], mm(GRIP_LAND));
            let mut dims = [side.clone(), side.clone(), side];
            dims[along] = format!("{} + 2 mm", size[along]);
            let cutter = format!("{body} grip");
            kit.push((
                "solid.box",
                json!({
                    "center": centre, "length": dims[0], "width": dims[1], "height": dims[2],
                    "name": format!("{cutter} cutter"), "body_name": cutter,
                }),
            ));
            kit.push((
                "solid.move",
                json!({
                    "bodies": [cutter], "origin": centre, "axis": axis, "angle": "45 deg",
                    "name": format!("{cutter} turn"),
                }),
            ));
            kit.push((
                "solid.combine",
                json!({
                    "target": body, "tools": [cutter], "operation": "cut", "name": cutter,
                }),
            ));
        }
        _ => {}
    }
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
    match place(at, front_h, floor) {
        "middle" => format!("({z0} + {z1}) / 2"),
        "top" => format!("{z1} - {end}"),
        _ => format!("{z0} + {end}"),
    }
}

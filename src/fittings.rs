//! The hardware the cabinet builders share: shelf pins on the 32 mm system, hinge cups, and
//! the screw holes for knobs and pulls. Sizes are common-practice values.

use crate::kit::{mm, Kit};
use serde_json::{json, Value};

pub const PITCH: f64 = 32.0; // 32 mm system: shelf pin holes
pub const PIN_SETBACK: f64 = 37.0; // pin rows, from the front and from the back of the shelf
pub const PIN_D: f64 = 5.0;
pub const DOOR_GAP: f64 = 2.0; // around overlay doors, and between door and carcass
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
            "solid.hole",
            json!({
                "position": rows.position, "direction": rows.into,
                "diameter": mm(PIN_D), "depth": rows.depth,
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

/// A hole in a door or a drawer front, from its back face.
pub fn hole(
    kit: &Kit,
    part: &str,
    position: Value,
    into: [i32; 3],
    d: f64,
    depth: &str,
    what: &str,
) {
    let name = &kit.name;
    kit.push((
        "solid.hole",
        json!({
            "position": position, "direction": into,
            "diameter": mm(d), "depth": depth,
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

// ponytail: where a door's handle sits is guessed from the cabinet: mid-height on a tall
// door, near the top on a floor unit (it has a plinth), else near the bottom as on a wall
// unit. A position field in the dialog if the guess is wrong too often.
/// The middle of the handle on a door from `z0` up to `z1`. `front_h` is the cabinet's height
/// above its plinth, in mm.
pub fn handle_height(handles: &str, z0: &str, z1: &str, front_h: f64, plinth: bool) -> String {
    let half = if handles == "pull" { PULL / 2.0 } else { 0.0 };
    let end = mm(HANDLE_END + half);
    match (front_h > 1200.0, plinth) {
        (true, _) => format!("({z0} + {z1}) / 2"),
        (_, true) => format!("{z1} - {end}"),
        _ => format!("{z0} + {end}"),
    }
}

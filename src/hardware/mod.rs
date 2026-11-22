//! The hinges' and drawer slides' sizes, shared by every box cabinet as design parameters
//! (`HW_cup_d`, ...), made the first time a cabinet needs them. Every hole a hinge or a slide
//! needs is a cylinder cut whose size and place are expressions over them, so changing one in
//! Parameters cuts every board again.
//!
//! A hinge is a cup in the door (`HW_cup_inset` in from its hinge edge) and a plate's two
//! screw holes in the side it hangs on (`HW_pitch` apart about the cup's height, `HW_setback`
//! behind the front of the opening, on the shelf pins' line). A slide is two screw holes in a
//! drawer box's side (`HW_setback` and `6*HW_pitch` further behind the box's front) and two in
//! the carcass side beside it (as far behind the front of the opening), in the middle of the
//! box side's height. The box is `2*HW_slide_t` narrower than the opening.

use crate::fittings::{CUP_CENTRE, CUP_D, CUP_DEPTH, PITCH};
use crate::kit::mm;
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

/// How deep a screw hole goes into a carcass side `thickness` thick: `HW_screw_depth`, but
/// never closer than 3 mm to its outer face.
pub fn screw_depth(thickness: &str) -> String {
    format!("min(HW_screw_depth, {thickness} - 3 mm)")
}

/// Whether a builder's commands use the shared parameters.
pub fn used(commands: &Commands) -> bool {
    commands.iter().any(|c| c.1.to_string().contains("HW_"))
}

#[cfg(test)]
mod tests;

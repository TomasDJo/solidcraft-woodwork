//! Reads and checks the dialog's fields. The numbers here only check the input.

use crate::fittings::PITCH;
use crate::kit::{check, num, Kit};
use serde_json::Value;

/// The checked input. Lengths are in mm.
pub struct Input<'a> {
    pub a: f64,
    pub b: f64,
    pub d: f64,
    pub h: f64,
    pub t: f64,
    pub plinth: f64,
    /// Depth at the return's end: `d` on the L.
    pub dr: f64,
    pub shelves: u32,
    pub pins: bool,
    /// Which end the doors hang on, if there are doors.
    pub on_return: Option<bool>,
    pub handles: &'a str,
    pub handle_at: &'a str,
    pub right: bool,
    pub diagonal: bool,
}

pub fn read<'a>(kit: &Kit, p: &'a Value) -> Result<Input<'a>, String> {
    let size = |key: &str, default_mm: f64| kit.size(p, key, default_mm);
    let (a, b, d, h) = (
        size("width", 900.0)?,
        size("return", 900.0)?,
        size("depth", 560.0)?,
        size("height", 720.0)?,
    );
    let t = size("thickness", 18.0)?;
    let plinth = size("plinth", 0.0)?;
    let shelves = num(p, "shelves", 1.0)?;
    let pins = p
        .get("shelf_pins")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let diagonal = match p.get("shape").and_then(Value::as_str).unwrap_or("L-shaped") {
        "L-shaped" => false,
        "diagonal" => true,
        _ => return Err("shape must be L-shaped or diagonal".into()),
    };
    let right = hand(p)? == "right";
    // Left and right as you face the corner: the return's end is on the corner's own side.
    let on_return = match p.get("doors").and_then(Value::as_str).unwrap_or("none") {
        "none" => None,
        "hinged left" => Some(!right),
        "hinged right" => Some(right),
        _ => return Err("doors must be none, hinged left or hinged right".into()),
    };
    let doors = on_return.is_some();
    let handles = p.get("handles").and_then(Value::as_str).unwrap_or("none");
    check(
        matches!(handles, "none" | "knob" | "pull"),
        "handles must be none, knob or pull",
    )?;
    let handle_at = crate::fittings::handle_position(p)?;

    check((3.0..=50.0).contains(&t), "thickness must be 3 to 50 mm")?;
    check(
        (0.0..=20.0).contains(&shelves) && shelves.fract() == 0.0,
        "shelves must be 0 to 20",
    )?;
    check(plinth >= 0.0, "plinth cannot be negative")?;
    check(
        a <= 5000.0 && b <= 5000.0 && h <= 5000.0,
        "sizes are limited to 5000 mm",
    )?;
    check(d >= 150.0 + t, "depth is too small")?;
    check(
        a >= d + 200.0 && b >= d + 200.0,
        "width and return must each be 200 mm more than the depth",
    )?;
    // A front at 45 degrees from the back leg's end reaches the return's end this far from
    // the side wall.
    let dr = if diagonal { a - b + d } else { d };
    check(
        dr >= 150.0 + t,
        "the return is too long for a diagonal front: at most width + depth - 150 mm - thickness",
    )?;
    let clear_h = h - plinth - 2.0 * t; // between bottom and top panels
    check(
        (clear_h - shelves * t) / (shelves + 1.0) >= 20.0,
        "height is too small for the plinth and shelves",
    )?;
    check(
        !doors || t >= 16.0,
        "doors with hinge cups need panels of 16 mm or more",
    )?;
    check(
        !doors || h - plinth >= 300.0,
        "too low for a door with two hinges",
    )?;
    check(
        !pins || t >= 12.0,
        "shelf pin holes need panels of 12 mm or more",
    )?;
    check(
        !pins || clear_h >= 4.0 * PITCH,
        "too low for shelf pin holes",
    )?;
    Ok(Input {
        a,
        b,
        d,
        h,
        t,
        plinth,
        dr,
        shelves: shelves as u32,
        pins,
        on_return,
        handles,
        handle_at,
        right,
        diagonal,
    })
}

/// Which corner of the room the cabinet is for, left or right, from its `corner` field. On
/// auto it follows `place`: against a cabinet's right side only a right corner makes sense,
/// its return coming forward at the far end of the run; anywhere else it is a left corner.
pub fn hand(p: &Value) -> Result<&'static str, String> {
    let placed_right = p.get("place").and_then(Value::as_str) == Some("right");
    match p.get("corner").and_then(Value::as_str).unwrap_or("auto") {
        "left" => Ok("left"),
        "right" => Ok("right"),
        "auto" if placed_right => Ok("right"),
        "auto" => Ok("left"),
        _ => Err("corner must be auto, left or right".into()),
    }
}

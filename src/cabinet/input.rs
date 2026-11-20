//! Reads and checks the dialog's fields. The numbers here only check the input.

use super::drawers::{BOX_CLEAR, SLIDE};
use super::{GROOVE_INSET, RAIL_D};
use crate::fittings::{DOOR_GAP, PITCH};
use crate::kit::{check, num, Kit};
use serde_json::Value;

/// The checked input. Lengths are in mm.
#[derive(Clone, Copy)]
pub struct Input<'a> {
    /// standard or sink (a sink base: rails instead of a top, no shelves).
    pub kind: &'a str,
    pub w: f64,
    pub h: f64,
    pub d: f64,
    pub t: f64,
    pub bt: f64,
    pub plinth: f64,
    pub shelves: f64,
    pub dividers: f64,
    pub doors: f64,
    pub drawers: f64,
    pub pins: bool,
    pub back: &'a str,
    pub handles: &'a str,
    pub handle_at: &'a str,
    pub hinge_right: bool, // the first door is hinged on its right
    /// Inset fronts sit inside the carcass opening; overlay ones cover its edges.
    pub inset: bool,
    /// The gap around and between the fronts.
    pub reveal: f64,
    /// One compartment's width, and the height between the bottom and top panels.
    pub cell_w: f64,
    pub clear_h: f64,
    /// Depth of the shelves, which stop at the back, and behind inset fronts.
    pub shelf_d: f64,
    /// Height of a drawer front: of each below the top one, and of the top one, which is
    /// `drawer_height` (0 when the drawers are all alike: no parameter then).
    pub front_h: f64,
    pub drawer_h: f64,
    /// With doors too (`mixed`), the drawers at the top or at the bottom, over a fixed rail.
    pub mixed: bool,
    pub drawers_top: bool,
    /// The doors' height, and the lowest drawer front's lower edge.
    pub door_h: f64,
    pub drawer_z: f64,
}

pub fn read<'a>(kit: &Kit, p: &'a Value) -> Result<Input<'a>, String> {
    let size = |key: &str, default_mm: f64| kit.size(p, key, default_mm);
    let kind = p.get("kind").and_then(Value::as_str).unwrap_or("standard");
    check(
        matches!(kind, "standard" | "sink"),
        "kind must be standard or sink",
    )?;
    let sink = kind == "sink";
    let (w, h, d) = (
        size("width", 600.0)?,
        size("height", 720.0)?,
        size("depth", 560.0)?,
    );
    let t = size("thickness", 18.0)?;
    let bt = size("back_thickness", 6.0)?;
    let plinth = size("plinth", 0.0)?;
    let shelves = num(p, "shelves", 1.0)?;
    let doors = num(p, "doors", 0.0)?;
    let pins = p
        .get("shelf_pins")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    // A sink base has no dividers: the sink and its pipes take the room. Its dialog hides
    // them; an Edit to a sink base drops the ones the cabinet remembers.
    let dividers = if sink { 0.0 } else { num(p, "dividers", 0.0)? };
    let drawers = num(p, "drawers", 0.0)?;
    check(
        (0.0..=8.0).contains(&drawers) && drawers.fract() == 0.0,
        "drawers must be 0 to 8",
    )?;
    // Its top front is a false front before the sink, so drawers come two or more, and alone.
    check(
        !sink || drawers == 0.0 || (drawers >= 2.0 && doors == 0.0),
        "a sink base has doors or drawers, not both, and 2 drawers or more (the top front is a false front)",
    )?;
    let dh = size("drawer_height", 0.0)?;
    check(dh >= 0.0, "drawer front height cannot be negative")?;
    // Drawers and doors: the drawers at the top or the bottom, 150 mm high unless asked.
    let mixed = drawers > 0.0 && doors > 0.0;
    let drawers_top = match p.get("drawers_at").and_then(Value::as_str).unwrap_or("top") {
        "top" => true,
        "bottom" => false,
        _ => return Err("drawers_at must be top or bottom".into()),
    };
    // A drawer cabinet is all drawers: no shelves behind them. Nor under a sink.
    let (shelves, pins) = if (drawers > 0.0 && !mixed) || sink {
        (0.0, false)
    } else {
        (shelves, pins)
    };
    let back = p.get("back").and_then(Value::as_str).unwrap_or("groove");
    let inset = match p.get("fronts").and_then(Value::as_str).unwrap_or("overlay") {
        "overlay" => false,
        "inset" => true,
        _ => return Err("fronts must be overlay or inset".into()),
    };
    let g = num(p, "reveal", DOOR_GAP)?; // mm, whatever the units
    check((0.5..=6.0).contains(&g), "the reveal must be 0.5 to 6 mm")?;

    check((3.0..=50.0).contains(&t), "thickness must be 3 to 50 mm")?;
    // Without a back its thickness is hidden in the dialog and not used.
    check(
        back == "none" || (2.0..=t).contains(&bt),
        "back thickness must be 2 mm up to the panel thickness",
    )?;
    check(
        (0.0..=20.0).contains(&shelves) && shelves.fract() == 0.0,
        "shelves must be 0 to 20",
    )?;
    check(plinth >= 0.0, "plinth cannot be negative")?;
    check(
        matches!(back, "none" | "overlay" | "groove"),
        "back must be none, overlay or groove",
    )?;
    check(
        w <= 5000.0 && h <= 5000.0 && d <= 5000.0,
        "sizes are limited to 5000 mm",
    )?;

    let inner_w = w - 2.0 * t;
    // How far inside the carcass's outside the fronts' opening is: inset fronts sit between
    // the panels.
    let o = if inset { t } else { 0.0 };
    // The fronts fill the opening: reveals round and between the doors and the drawer fronts.
    let (room, n) = (h - plinth - 2.0 * o, drawers);
    let drawer_h = match (mixed, drawers > 1.0) {
        (true, _) if dh == 0.0 => 150.0,
        (true, _) | (_, true) => dh, // the top drawer; the rest share what is left
        _ => 0.0,
    };
    let (front_h, door_h) = if mixed {
        (drawer_h, room - n * drawer_h - (n + 2.0) * g)
    } else if drawer_h > 0.0 {
        (
            (room - (n + 1.0) * g - drawer_h) / (n - 1.0),
            room - 2.0 * g,
        )
    } else {
        ((room - (n + 1.0) * g) / n.max(1.0), room - 2.0 * g)
    };
    let drawer_z = match mixed && drawers_top {
        true => h - o - n * (drawer_h + g),
        false => plinth + o + g,
    };
    // Between the bottom and top panels, or between the rail (centred behind the reveal
    // between the doors and the drawers) and the panel at the doors' other end.
    let clear_h = match mixed {
        true => door_h + o + 1.5 * g - 1.5 * t,
        false => h - plinth - 2.0 * t,
    };
    let gap = (clear_h - shelves * t) / (shelves + 1.0);
    check(
        (0.0..=10.0).contains(&dividers) && dividers.fract() == 0.0,
        "dividers must be 0 to 10",
    )?;
    // Dividers split the inside into equal compartments.
    let cell_w = (inner_w - dividers * t) / (dividers + 1.0);
    check(
        cell_w >= 50.0,
        "width is too small for the panels and dividers",
    )?;
    // The app runs at most 256 commands per plugin call.
    check(
        (dividers + 1.0) * shelves <= 200.0,
        "too many shelves: 200 at most in one cabinet",
    )?;
    check(
        gap >= 20.0,
        "height is too small for the plinth and shelves",
    )?;
    check(d >= 150.0 + t, "depth is too small")?;
    // More than two doors need a divider behind every hinge.
    check(
        [0.0, 1.0, 2.0, dividers + 1.0].contains(&doors),
        "doors must be 0, 1, 2 or one per compartment (dividers + 1)",
    )?;
    check(
        drawers == 0.0 || dividers == 0.0,
        "drawers cannot be combined with dividers",
    )?;
    // Room for a box behind each front (an inset one beside doors stays clear of the rail
    // as an overlay one does), and the width of the box between the slides.
    let lowest = match inset && !mixed {
        true => 2.0 * BOX_CLEAR + 50.0,
        false => 2.0 * (t + BOX_CLEAR) + 50.0,
    };
    check(
        drawers == 0.0 || front_h >= lowest,
        "too low for that many drawers",
    )?;
    check(
        drawer_h == 0.0 || drawer_h >= lowest,
        "the drawer front height is too low for a drawer box",
    )?;
    check(
        drawers == 0.0 || inner_w - 2.0 * SLIDE >= 100.0,
        "too narrow for drawers",
    )?;
    check(
        doors == 0.0 || t >= 16.0,
        "doors with hinge cups need panels of 16 mm or more",
    )?;
    let handles = p.get("handles").and_then(Value::as_str).unwrap_or("none");
    check(
        matches!(handles, "none" | "knob" | "pull"),
        "handles must be none, knob or pull",
    )?;
    let handle_at = crate::fittings::handle_position(p)?;
    let hinge_right = match p
        .get("door_hinge")
        .and_then(Value::as_str)
        .unwrap_or("left")
    {
        "left" => false,
        "right" => true,
        _ => return Err("door_hinge must be left or right".into()),
    };
    check(
        handles == "none" || doors == 0.0 || (w - 2.0 * o - (doors + 1.0) * g) / doors >= 150.0,
        "doors are too narrow for handles",
    )?;
    check(
        doors == 0.0 || door_h + 2.0 * g >= 300.0,
        "too low for a door with two hinges",
    )?;
    check(
        !pins || t >= 12.0,
        "shelf pin holes need panels of 12 mm or more",
    )?;
    // Pin holes sit on a 32 mm grid above the bottom panel, two pitches clear of both ends.
    check(
        !pins || clear_h >= 4.0 * PITCH,
        "too low for shelf pin holes",
    )?;
    let shelf_d = match back {
        "overlay" => d - bt,
        "groove" => d - GROOVE_INSET - bt,
        _ => d,
    } - o;
    // A sink base's two rails run from the front and from where the shelves' depth ends.
    check(
        !sink || shelf_d + o >= 2.0 * RAIL_D,
        "too shallow for a sink base: its two 100 mm rails would overlap",
    )?;
    Ok(Input {
        kind,
        w,
        h,
        d,
        t,
        bt,
        plinth,
        shelves,
        dividers,
        doors,
        drawers,
        pins,
        back,
        handles,
        handle_at,
        hinge_right,
        inset,
        reveal: g,
        cell_w,
        clear_h,
        shelf_d,
        front_h,
        drawer_h,
        mixed,
        drawers_top,
        door_h,
        drawer_z,
    })
}

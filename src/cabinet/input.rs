//! Reads and checks the dialog's fields. The numbers here only check the input.

use super::drawers::{BOX_CLEAR, SLIDE};
use super::GROOVE_INSET;
use crate::fittings::{DOOR_GAP, PITCH};
use crate::kit::{check, num, Kit};
use serde_json::Value;

/// The checked input. Lengths are in mm.
#[derive(Clone, Copy)]
pub struct Input<'a> {
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
    /// Where the doors' top edge is: under the drawers, on the plinth when the drawers fill
    /// the front.
    pub door_top: f64,
}

pub fn read<'a>(kit: &Kit, p: &'a Value) -> Result<Input<'a>, String> {
    let size = |key: &str, default_mm: f64| kit.size(p, key, default_mm);
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
    let dividers = num(p, "dividers", 0.0)?;
    let pins = p
        .get("shelf_pins")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let drawers = num(p, "drawers", 0.0)?;
    check(
        (0.0..=8.0).contains(&drawers) && drawers.fract() == 0.0,
        "drawers must be 0 to 8",
    )?;
    let dh = size("drawer_height", 0.0)?;
    check(dh >= 0.0, "drawer front height cannot be negative")?;
    // Drawers over doors: the drawers at the top, 150 mm high unless asked, the doors below.
    let over = drawers > 0.0 && doors > 0.0;
    // A drawer cabinet is all drawers: no shelves behind them.
    let (shelves, pins) = if drawers > 0.0 && !over {
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
    check(
        (2.0..=t).contains(&bt),
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
    // the panels, over the drawers a rail as thick as a panel.
    let o = if inset { t } else { 0.0 };
    let rail_lap = if inset { t + g } else { 0.0 };
    let drawer_h = if over && dh == 0.0 {
        150.0
    } else if over || drawers > 1.0 {
        dh // the top drawer; the rest share what is left
    } else {
        0.0
    };
    let front_room = h - plinth - 2.0 * o; // the fronts' height, and the gaps
    let (front_h, door_top) = if over {
        (drawer_h, h - o - g - drawers * (drawer_h + g) - rail_lap)
    } else if drawer_h > 0.0 {
        let rest = front_room - (drawers + 1.0) * g - drawer_h;
        (rest / (drawers - 1.0), plinth + o)
    } else if drawers > 0.0 {
        ((front_room - (drawers + 1.0) * g) / drawers, plinth + o)
    } else {
        (0.0, h - o - g)
    };
    // Between the bottom panel and the top panel, or the rail under the drawers: behind the
    // gap over the doors, or under the drawers between inset fronts.
    let rail_z = match inset {
        true => door_top + g,
        false => door_top + g / 2.0 - t / 2.0,
    };
    let clear_h = match over {
        true => rail_z - plinth - t,
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
    // Room for a drawer box behind each front, and the width of the box between the slides.
    let lowest = 2.0 * (t - o + BOX_CLEAR) + 50.0;
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
        doors == 0.0 || door_top + g - plinth - o >= 300.0,
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
    Ok(Input {
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
        door_top,
    })
}

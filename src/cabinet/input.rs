//! Reads and checks the dialog's fields. The numbers here only check the input.

use super::drawers::{BOX_BEHIND, BOX_CLEAR, SLIDE};
use super::{GROOVE_INSET, RAIL_D};
use crate::fittings::{DOOR_GAP, PITCH};
use crate::kit::{check, num, Kit};
use serde_json::Value;

// ponytail: the shortest slide for the default `HW_` sizes; a slide's holes changed in
// Parameters are not checked.
const MIN_SLIDE: f64 = 250.0;
/// The lowest drawer box side or back, in mm: a slide's members are 45 mm high (#221).
const MIN_BOX_SIDE: f64 = 50.0;

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
    /// board, notch, legs or casters (`fittings::plinth_type`).
    pub plinth_type: &'a str,
    /// How far the plinth board or the sides' notch is back from the front.
    pub plinth_setback: f64,
    pub shelves: f64,
    pub dividers: f64,
    /// Each divider two boards side by side, where overlay doors meet on it (#176).
    pub double: bool,
    pub doors: f64,
    pub drawers: f64,
    pub pins: bool,
    /// The shelves fixed and joined (no shelf pins: the inside's joinery).
    pub fixed: bool,
    pub back: &'a str,
    /// none, cleat (French cleat), rail (a hanging rail) or steel (bought rail and hangers).
    pub wall_mount: &'a str,
    /// The mount board's thickness and height (a cleat's on its rear face): 0 without one.
    pub mount_t: f64,
    pub mount_h: f64,
    pub handles: &'a str,
    /// The doors' handles (`door_handles`); `handles` is the drawer fronts'.
    pub door_handles: &'a str,
    pub handle_at: &'a str,
    /// Grooves cut into the fronts' faces (`front_style`, #182): `None` for flat fronts.
    pub grooves: Option<crate::fittings::Grooves<'a>>,
    /// Shaker fronts (`front_style` shaker, #183): the frame's width and the panel's thickness.
    pub frame: Option<[f64; 2]>,
    /// Glass doors (`front_style` glass): the frame round a pane; drawer fronts get a panel.
    pub glass: bool,
    /// The drawer fronts take the frame (Shaker or glass): every one tall and wide enough for
    /// it (`fittings::framed`), and not integrated.
    pub framed_drawers: bool,
    /// A grip edge's gap (`grip_gap`, #230), in mm; `None` without grip edges.
    pub grip_gap: Option<f64>,
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
    /// The drawer boxes' boards (sides, front and back) and their bottoms.
    pub box_t: [f64; 2],
    /// The box's side and back heights in mm, 0 for the full height and the sides' (#221).
    pub box_h: [f64; 2],
    /// The drawer bottom's tab with Drawer handles Bottom tab (#181), in mm.
    pub tab_w: f64,
    /// Wooden runners (`slide_type` wood): the bottom runs in a groove in each side, this deep,
    /// with this much play round it. `None` on metal slides.
    pub runner: Option<[f64; 2]>,
    /// Strip runners (`slide_type` strips, #146): a strip this thick and this high on each side
    /// under the bottom, which is wider than the box and slides on them. With `runner` for the
    /// play. `None` otherwise.
    pub strip: Option<[f64; 2]>,
    /// Undermount slides (`slide_type` under_partial or under_full).
    pub under: bool,
    /// Finger joints at the drawer boxes' corners (`drawer_joints`, #131).
    pub box_fingers: bool,
    /// The box's own front board is the drawer front (`drawer_front` integrated, #163).
    pub integrated: bool,
    pub drawers_top: bool,
    /// The doors' height, and the lowest drawer front's lower edge.
    pub door_h: f64,
    pub drawer_z: f64,
    /// The drawer slides' length: mm, and with its unit (`450 mm`), from `slide_length`
    pub slide: (f64, &'a str),
    /// The slides' type (`hardware::SLIDE_TYPES`): its name in the cut list, and how far it
    /// pulls out, in mm
    pub slide_type: (&'a str, f64),
}

/// The lowest drawer front height but 0 (Auto), in mm: the smallest a drawer box fits behind.
pub const MIN_DRAWER_FRONT: f64 = 100.0;

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
    let plinth_type = crate::fittings::plinth_type(p, plinth)?;
    let plinth_setback = size("plinth_setback", crate::fittings::KICK_RECESS)?;
    let shelves = num(p, "shelves", 1.0)?;
    let doors = num(p, "doors", 0.0)?;
    let pins = p
        .get("shelf_pins")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    // Without pins the shelves are fixed, joined by the inside's joinery (none on a cabinet
    // that remembers no method: as before).
    let fixed = !pins;
    let drawers = num(p, "drawers", 0.0)?;
    check(
        (0.0..=8.0).contains(&drawers) && drawers.fract() == 0.0,
        "drawers must be 0 to 8",
    )?;
    // A sink base takes doors or drawers: with doors the dialog hides Drawers, so a value
    // still sent (or remembered by Edit) is not used.
    let drawers = if sink && doors > 0.0 { 0.0 } else { drawers };
    // Its top front is a false front before the sink, so drawers come two or more.
    check(
        !sink || drawers == 0.0 || drawers >= 2.0,
        "a sink base has 2 drawers or more (the top front is a false front)",
    )?;
    // A sink base has no dividers (the sink and its pipes take the room), nor drawers beside
    // dividers: the dialogs hide Dividers then, and a value still sent is not used.
    let dividers = match sink || drawers > 0.0 {
        true => 0.0,
        false => num(p, "dividers", 0.0)?,
    };
    let box_t = [size("drawer_thickness", 12.0)?, size("drawer_bottom", 6.0)?];
    // The box's sides and back, lower than the box when asked (#221): 0 is the full height,
    // and the back's 0 the sides'.
    let box_h = [
        size("drawer_side_height", 0.0)?,
        size("drawer_back_height", 0.0)?,
    ];
    let tab_w = size("tab_width", 120.0)?;
    check(
        p.get("handles").and_then(Value::as_str) != Some("tab") || (40.0..=600.0).contains(&tab_w),
        "the tab width must be 40 to 600 mm",
    )?;
    let box_fingers = match p.get("drawer_joints").and_then(Value::as_str) {
        None | Some("none") => false,
        Some("finger") => true,
        _ => return Err("drawer_joints must be none or finger".into()),
    };
    let integrated = match p.get("drawer_front").and_then(Value::as_str) {
        None | Some("separate") => false,
        Some("integrated") => true,
        _ => return Err("drawer_front must be separate or integrated".into()),
    };
    // the slides' screws go 8 mm into the box's sides (`HW_box_screw_depth`)
    check(
        (10.0..=25.0).contains(&box_t[0]) && (3.0..=19.0).contains(&box_t[1]),
        "drawer box boards must be 10 to 25 mm, and their bottoms 3 to 19 mm",
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
        plinth == 0.0
            || matches!(plinth_type, "legs" | "casters")
            || (0.0..d - t).contains(&plinth_setback),
        "the plinth set-back must be 0 up to the depth less a panel thickness",
    )?;
    check(
        matches!(back, "none" | "overlay" | "groove"),
        "back must be none, overlay or groove",
    )?;
    check(
        w <= 5000.0 && h <= 5000.0 && d <= 5000.0,
        "sizes are limited to 5000 mm",
    )?;

    let between = h - plinth - 2.0 * t; // between the bottom and the top
    let (wall_mount, mount_t, mount_h) = crate::fittings::wall_mount(kit, p, t, between)?;
    // An overlaid back would hold the cabinet off the wall by its thickness.
    check(
        mount_t == 0.0 || back != "overlay",
        "a wall mount board needs a back in a groove or none",
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
    // Overlay doors that meet on every divider (one door a compartment) can stand on double
    // dividers, two boards side by side, so each door covers a whole board and takes full
    // overlay hinges (#176). Anywhere else the setting is not used.
    let double = matches!(p.get("double_dividers"), Some(v) if v == "yes" || v == true)
        && !inset
        && dividers > 0.0
        && doors == dividers + 1.0;
    // Dividers split the inside into equal compartments.
    let boards = if double { 2.0 * dividers } else { dividers };
    let cell_w = (inner_w - boards * t) / (dividers + 1.0);
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
        &format!(
            "doors must be 0, 1, 2 or one per compartment (dividers + 1 = {} here)",
            dividers + 1.0
        ),
    )?;
    // Room for a box behind each front (an inset one beside doors stays clear of the rail
    // as an overlay one does), and the width of the box between the slides.
    let box_front = match inset && !mixed {
        true => 2.0 * BOX_CLEAR + 50.0,
        false => 2.0 * (t + BOX_CLEAR) + 50.0,
    };
    // A drawer front height is 0 (Auto) or at least 100 mm, more where thick boards leave
    // no room for a box, rounded up to a whole mm or 1/8 in. The dialog's minimum stays 0.
    let step = if kit.unit == "in" { 25.4 / 8.0 } else { 1.0 };
    let floor = (box_front.max(MIN_DRAWER_FRONT) / step - 1e-6).ceil() * step;
    check(
        dh == 0.0 || drawer_h == 0.0 || drawer_h >= floor - 1e-6,
        &format!(
            "the drawer front height must be 0 (Auto) or at least {}",
            kit.length(floor)
        ),
    )?;
    check(
        drawers == 0.0 || front_h >= box_front,
        "too low for that many drawers",
    )?;
    check(
        drawer_h == 0.0 || drawer_h >= box_front,
        "the drawer front height is too low for a drawer box",
    )?;
    // The sides and back: 0, or at least a slide's members (45 mm) and the bottom with an
    // undermount slide's recess. Taller is not refused: each drawer's boards are capped by its
    // box and its sides (`drawers`), as a taller value in Parameters is, so Edit of such a
    // cabinet still builds.
    for (value, what) in box_h.iter().zip(["side", "back"]) {
        check(
            drawers == 0.0 || *value == 0.0 || *value >= MIN_BOX_SIDE,
            &format!(
                "the drawer {what} height must be 0 or at least {}",
                kit.length(MIN_BOX_SIDE)
            ),
        )?;
    }
    check(
        drawers == 0.0 || inner_w - 2.0 * SLIDE >= 100.0,
        "too narrow for drawers",
    )?;
    // the slides' screw holes stop 3 mm short of the outside (`hardware::screw_depth`)
    check(
        drawers == 0.0 || t >= 6.0,
        "drawer slides need panels of 6 mm or more",
    )?;
    check(
        doors == 0.0 || t >= 16.0,
        "doors with hinge cups need panels of 16 mm or more",
    )?;
    let handles = p.get("handles").and_then(Value::as_str).unwrap_or("none");
    check(
        matches!(
            handles,
            "none" | "knob" | "pull" | "grip" | "groove" | "tab"
        ),
        "handles must be none, knob, pull, grip, groove or tab",
    )?;
    crate::fittings::profile_fits(handles, t)?;
    let door_handles = crate::fittings::door_handles(p)?;
    crate::fittings::profile_fits(door_handles, t)?;
    // only checked on a cabinet with fronts to cut them into
    let grooves = match doors > 0.0 || drawers > 0.0 {
        true => crate::fittings::grooves(p, size, t, doors > 0.0)?,
        false => None,
    };
    let door_w = (w - 2.0 * o - (doors + 1.0) * g) / doors.max(1.0);
    let smallest = (doors > 0.0).then_some(door_w.min(door_h));
    let frame = match doors > 0.0 || drawers > 0.0 {
        true => crate::fittings::frame(p, size, t, smallest, drawers > 0.0)?,
        false => None,
    };
    let handle_at = crate::fittings::handle_position(p)?;
    let low = if drawer_h > 0.0 {
        front_h.min(drawer_h)
    } else {
        front_h
    };
    let wide = w - 2.0 * o - 2.0 * g;
    let framed_drawers = drawers > 0.0
        && !integrated
        && frame.is_some_and(|[fw, _]| low.min(wide) >= crate::fittings::framed(fw));
    let glass = p.get("front_style").and_then(Value::as_str) == Some("glass");
    // a glass drawer front's handle goes in the middle of its top rail (auto too) or bottom rail
    check(
        !(glass && framed_drawers && matches!(handles, "knob" | "pull") && handle_at == "middle"),
        "a glass drawer front's handle must be at the top or the bottom (Handle position): in the middle it would go through the glass",
    )?;
    // the fronts with a grip edge: the doors, the drawers, or both
    let mut gripped = Vec::new();
    if doors > 0.0 && door_handles == "grip" {
        gripped.push(door_h);
    }
    if drawers > 0.0 && handles == "grip" {
        gripped.extend([front_h, drawer_h]);
    }
    let lowest = gripped
        .into_iter()
        .filter(|&h| h > 0.0)
        .fold(f64::MAX, f64::min);
    let grip_gap = match lowest < f64::MAX {
        true => crate::fittings::grip_gap("grip", handle_at, size, lowest)?,
        false => None,
    };
    // a grip edge's drawer box is the gap lower too: it must still fit (`box_front`); a sink
    // base's top front is a false front with no box (the rest are `front_h`)
    let lowest_drawer = match drawer_h > 0.0 && !sink {
        true => front_h.min(drawer_h),
        false => front_h,
    };
    check(
        grip_gap.is_none_or(|gap| {
            drawers == 0.0 || handles != "grip" || lowest_drawer - gap >= box_front
        }),
        "the drawer fronts are too low for that grip gap: a drawer box would be under 50 mm",
    )?;
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
        door_handles == "none"
            || doors == 0.0
            || (w - 2.0 * o - (doors + 1.0) * g) / doors >= 150.0,
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
        // in front of a mount board, the back moves forward by its thickness
        "groove" if mount_t > 0.0 => d - mount_t - bt,
        "groove" => d - GROOVE_INSET - bt,
        _ => d - mount_t,
    } - o;
    // A sink base's two rails run from the front and from where the shelves' depth ends.
    check(
        !sink || shelf_d + o >= 2.0 * RAIL_D,
        "too shallow for a sink base: its two 100 mm rails would overlap",
    )?;
    // The slides' length, and so their screw holes, from the box's depth (`crate::hardware`).
    let box_d = if sink {
        shelf_d / 2.0
    } else {
        shelf_d - BOX_BEHIND
    };
    let kind_of = p
        .get("slide_type")
        .and_then(Value::as_str)
        .unwrap_or("full");
    let strips = kind_of == "strips";
    let strip = [size("strip_thickness", 12.0)?, size("strip_height", 18.0)?];
    check(
        !strips
            || drawers == 0.0
            || ((6.0..=30.0).contains(&strip[0]) && (10.0..=60.0).contains(&strip[1])),
        "the runner strips must be 6 to 30 mm thick and 10 to 60 mm high",
    )?;
    // the bottom must bear on the strips: less play than a strip is thick
    let play = size("runner_play", 1.0)?;
    check(
        !strips || drawers == 0.0 || (0.0..strip[0]).contains(&play),
        "the runner play must be less than the strips are thick",
    )?;
    // Dado runners (`wood`), and strip runners in the same way but on strips, not in grooves:
    // no slides, no stop.
    let grooved = kind_of == "wood";
    let wood = grooved || strips;
    let runner = [size("runner_depth", 6.0)?, size("runner_play", 1.0)?];
    check(
        !grooved || drawers == 0.0 || ((3.0..=t - 6.0).contains(&runner[0]) && (0.0..runner[0]).contains(&runner[1])),
        "the runner groove must be 3 mm deep up to the panel thickness less 6 mm, with less play than its depth",
    )?;
    // Undermount (Blum's rule): the box 42 mm narrower inside than the opening, which leaves
    // room for sides of 16 mm at most; the box as long as the slide, which fits the inside.
    let under = kind_of.starts_with("under_");
    check(
        !under || drawers == 0.0 || box_t[0] <= 16.0,
        "undermount slides need drawer box sides of 16 mm or less",
    )?;
    check(
        wood || under || drawers == 0.0 || box_d >= MIN_SLIDE,
        "too shallow for drawers: the boxes need 250 mm slides at least",
    )?;
    let field = p.get("slide_length").and_then(Value::as_str);
    let field = field.unwrap_or("auto");
    let slide = match (drawers > 0.0 && !wood, under) {
        (true, false) => crate::hardware::slide_length(field, kit.unit, box_d)?,
        (true, true) => {
            let inside = if sink { shelf_d / 2.0 } else { shelf_d };
            crate::hardware::under_length(field, kit.unit, inside)?
        }
        (false, _) => (0.0, ""),
    };
    let named = crate::hardware::SLIDE_TYPES.iter().find(|t| t.0 == kind_of);
    let named = named
        .ok_or("slide_type must be partial, full, over, wood, strips, under_partial or under_full")?
        .1;
    // A wooden runner has no stop: the drawer comes out two thirds of its box's depth.
    let travel = match wood {
        true => (box_d * 2.0 / 3.0).round(),
        false => crate::hardware::travel(kind_of, slide.0),
    };
    let slide_type = (named, travel);
    // An integrated front is only as wide as the box (#223), so a pull's two holes must fit it:
    // the box as `drawers` makes it for each slide type.
    let box_w = match (strips, grooved, under) {
        (true, _, _) => inner_w - 2.0 * (strip[0] + play),
        (_, true, _) => inner_w - 2.0 * runner[1],
        (_, _, true) => inner_w - 42.0 + 2.0 * box_t[0],
        _ => inner_w - 2.0 * SLIDE,
    };
    check(
        !(integrated && drawers > 0.0 && handles == "pull") || box_w >= 150.0,
        "integrated drawer fronts are too narrow for pulls",
    )?;
    // a bottom tab's two rounded corners must fit the box inside
    let inside = box_w - 2.0 * box_t[0];
    check(
        !(drawers > 0.0 && handles == "tab") || inside > 2.0 * crate::fittings::TAB_R,
        &format!("the drawer box is {inside:.0} mm inside, too narrow for a bottom tab"),
    )?;
    Ok(Input {
        kind,
        w,
        h,
        d,
        t,
        bt,
        plinth,
        plinth_type,
        plinth_setback,
        shelves,
        dividers,
        double,
        doors,
        drawers,
        pins,
        fixed,
        back,
        wall_mount,
        mount_t,
        mount_h,
        handles,
        door_handles,
        handle_at,
        grooves,
        frame,
        glass,
        framed_drawers,
        grip_gap,
        hinge_right,
        inset,
        reveal: g,
        cell_w,
        clear_h,
        shelf_d,
        front_h,
        drawer_h,
        mixed,
        box_t,
        box_h,
        tab_w,
        runner: wood.then_some(runner),
        strip: strips.then_some(strip),
        under,
        box_fingers,
        integrated,
        drawers_top,
        door_h,
        drawer_z,
        slide,
        slide_type,
    })
}

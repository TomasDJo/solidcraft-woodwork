//! Box carcass. X is width, Y is depth (front at 0), Z is height.
//!
//! The sizes become design parameters (`Name_width`, ...) and every panel and hole is an
//! expression over them, so the cabinet follows when a parameter is changed later. The numbers
//! in `input.rs` only check the input; the model itself is all expressions.
//!
//! This file is the carcass and the order of things; `doors.rs` and `drawers.rs` are the fronts.

mod doors;
mod drawers;
mod input;

use crate::fittings::{self, PinRows, CLIP, HANDLE_D, LEG_INSET, PIN_SETBACK, PITCH, STRIP};
use crate::joint::{Joinery, Joint, BOTTOM, CARCASS, INNER};
use crate::kit::{check, mm, Kit};
use crate::Commands;
use input::Input;
use serde_json::{json, Value};

const GROOVE_INSET: f64 = 12.0; // back groove, from the rear edge
const RAIL_D: f64 = 100.0; // a sink base's rails, front to back

/// What the steps of a build share: the checked input, and the sizes as expressions.
struct Cabinet<'a> {
    kit: &'a Kit,
    input: Input<'a>,
    joinery: Joinery<'a>,
    ew: String,
    eh: String,
    ed: String,
    et: String,
    ebt: String,
    epl: String,
    eiw: String, // inner width
    ech: String, // clear height
    // Depth of the sides, top and bottom, and of the shelves (which stop at the back, and
    // behind inset fronts).
    eside_d: String,
    eshelf_d: String,
    // The board the cabinet hangs by (a cleat or a hanging rail): its thickness and height.
    emount: Option<[String; 2]>,
    // The reveal around the fronts; how far the fronts' opening is inside the carcass's
    // outside (a panel when inset, which is also where the shelves start); and the y of the
    // fronts' front and back faces.
    er: String,
    /// Grooved fronts' spacing and depth parameters, on a cabinet with fronts.
    egrooves: Option<[String; 2]>,
    /// Shaker fronts' frame width and panel thickness parameters.
    eframe: Option<[String; 2]>,
    /// A grip edge's gap, the parameter `<Prefix>_grip_gap` (#230).
    egap: Option<String>,
    ein: String,
    efront: String,
    eback: String,
    ecell: String, // one compartment's width
    // The doors' lower and upper edges and height; the lowest drawer front's lower edge; the
    // top drawer's front height when it is a parameter; and with doors and drawers, the
    // fixed rail's lower face, behind the reveal between them.
    edoor: [String; 3],
    edraw: String,
    edh: Option<String>,
    erail: Option<String>,
}

/// Sizing by the drawers' inside (`sizing: drawer`): the width and depth that give the boxes
/// `drawer_width` x `drawer_depth` inside, written into `p` as its outside size. `sizing`
/// stays, so the cabinet remembers it: Edit and Apply settings work its outside out again
/// (`rebuild`), and its drawers keep their inside whatever changed (slides, boards, reveal).
pub fn from_drawers(p: &mut Value) -> Result<(), String> {
    // Match selected gives the sizes (the dialog hides Sizing then): outside sizing.
    if p["sizing"] != "drawer" || !p["_match"].is_null() {
        p["sizing"] = json!("outside");
        return Ok(());
    }
    let kit = Kit::new(p)?;
    let inside = [
        kit.size(p, "drawer_width", 0.0)?,
        kit.size(p, "drawer_depth", 0.0)?,
    ];
    check(
        inside.iter().all(|v| *v >= 50.0),
        "the drawers' inside width and depth must be 50 mm or more",
    )?;
    // The depth outside the shelves' (back, mount board, inset front) from a deep trial.
    let scale = if kit.unit == "in" { 25.4 } else { 1.0 };
    let mut trial = p.clone();
    trial["depth"] = json!(1000.0 / scale);
    trial["width"] = json!(1000.0 / scale);
    let input = input::read(&kit, &trial)?;
    check(input.drawers > 0.0, "drawer inside size needs drawers")?;
    let [bt, _] = input.box_t;
    // On strip runners the box also stands a strip's thickness off each side (#267).
    let strip = input.strip.map_or(0.0, |[st, _]| st);
    let clear = input
        .runner
        .map_or(drawers::SLIDE, |[_, play]| play + strip);
    let w = match input.under {
        true => inside[0] + 42.0 + 2.0 * input.t, // Blum's rule
        false => inside[0] + 2.0 * (bt + clear + input.t),
    };
    // An integrated front (#163) stands before the box: one box board in the depth, not two.
    let mut box_d = inside[1] + if input.integrated { bt } else { 2.0 * bt };
    let mut shelf_d = match (input.kind, input.runner) {
        ("sink", _) => 2.0 * box_d,
        // on runners the box reaches the back (#257), from a reveal before an overlay carcass
        (_, Some(_)) => box_d - if input.inset { 0.0 } else { input.reveal },
        _ => box_d + drawers::BOX_BEHIND,
    };
    if input.under {
        // the box is as long as a standard undermount slide: the shortest that holds it
        let mut ours = (crate::hardware::UNDER.iter()).filter(|s| s.0.ends_with(kit.unit));
        let slide = ours.find(|s| s.1 >= box_d - 1e-9);
        box_d = slide.ok_or("no standard undermount slide is that long")?.1;
        shelf_d = box_d * if input.kind == "sink" { 2.0 } else { 1.0 } + 3.0;
    }
    let d = shelf_d + 1000.0 - input.shelf_d;
    let round = |mm: f64| (mm / scale * 1e6).round() / 1e6;
    p["width"] = json!(round(w));
    p["depth"] = json!(round(d));
    Ok(())
}

/// The build's checks of the fields, without making anything: the dialog hook's (a whole build
/// costs too much of its fuel).
pub fn checked(p: &Value) -> Result<(), String> {
    let mut p = p.clone();
    from_drawers(&mut p)?;
    input::read(&Kit::new(&p)?, &p).map(|_| ())
}

pub fn build(p: &Value) -> Result<(Commands, Value), String> {
    let kit = Kit::new(p)?;
    let input = input::read(&kit, p)?;
    let Input {
        w,
        h,
        d,
        back,
        drawers,
        doors,
        ..
    } = input;

    let parameter = |key: &str, mm_value: f64, comment: &str| kit.parameter(key, mm_value, comment);
    let ew = parameter("width", w, "outside width");
    let eh = parameter("height", h, "outside height, plinth included");
    let ed = parameter("depth", d, "outside depth, back included");
    let et = parameter("thickness", input.t, "panel thickness");
    // Dividers, the rail between drawers and doors and fixed shelves take the second method.
    let inner = input.dividers > 0.0 || input.mixed || (input.fixed && input.shelves > 0.0);
    // The sides reach the floor below a raised bottom (not on legs or casters).
    let closed = input.plinth > 0.0 && !matches!(input.plinth_type, "legs" | "casters");
    let joinery = Joinery::new(&kit, p, (input.t, &et), inner, closed)?;
    let side_mm = if back == "overlay" { d - input.bt } else { d };
    joinery.fits(CARCASS, side_mm, p, "the sides")?;
    joinery.fits(BOTTOM, side_mm, p, "the sides")?;
    joinery.fits(INNER, input.shelf_d, p, "the dividers and the rail")?;
    // a divider between fixed shelves in dados is grooved from both sides at the same height
    let both = input.fixed && input.shelves > 0.0 && input.dividers > 0.0 && !input.double;
    check(
        !both || joinery.method(INNER) != "dado" || 2.0 * kit.size(p, "dado_depth", 6.0)? <= input.t - 3.0,
        "`dado_depth`: fixed shelves on both sides of a divider would groove it through; at most (thickness - 3 mm) / 2",
    )?;
    let ebt = match back {
        "none" => mm(0.0),
        _ => parameter("back_thickness", input.bt, "back panel thickness"),
    };
    // A plinth taken away sets its parameter to 0: Edit and Apply settings read the sizes from
    // the parameters (`edit::merge`), so a stale height would bring the plinth back.
    let epl = match input.plinth > 0.0 {
        true => parameter("plinth", input.plinth, "plinth height"),
        false if kit.has("plinth") => {
            parameter("plinth", 0.0, "plinth height");
            mm(0.0)
        }
        false => mm(0.0),
    };
    let emount = fittings::mount_sizes(&kit, input.wall_mount, input.mount_t, input.mount_h, &et);
    fittings::steel_rail(&kit, "wall_rail", input.wall_mount == "steel", w, &ew);
    // The back stands in front of a mount board, in its groove or with no back.
    let inset = emount.as_ref().map_or(mm(GROOVE_INSET), |m| m[0].clone());
    let (eside_d, eshelf_d) = match (back, &emount) {
        ("overlay", _) => (format!("({ed} - {ebt})"), format!("({ed} - {ebt})")),
        ("groove", _) => (ed.clone(), format!("({ed} - {inset} - {ebt})")),
        (_, Some([tm, _])) => (ed.clone(), format!("({ed} - {tm})")),
        _ => (ed.clone(), ed.clone()),
    };
    // Only with fronts, or when an edit took them off: the next edit that puts them back
    // takes the value up again. In mm whatever the dialog's unit.
    let er = match doors > 0.0 || drawers > 0.0 || kit.has("reveal") {
        true => kit.parameter_as(
            "reveal",
            input.reveal,
            "gap around the doors and drawer fronts",
            mm(input.reveal),
        ),
        false => mm(input.reveal),
    };
    let egrooves = (input.grooves).map(|g| fittings::groove_sizes(&kit, &g));
    let eframe = (input.frame).map(|f| fittings::frame_sizes(&kit, f));
    let egap = (input.grip_gap).map(|g| kit.parameter("grip_gap", g, "grip edge gap"));
    let (ein, efront, eback) = match input.inset {
        true => (et.clone(), mm(0.0), et.clone()),
        false => (mm(0.0), format!("-{er} - {et}"), format!("-{er}")),
    };
    let eshelf_d = match input.inset {
        true => format!("({eshelf_d} - {et})"),
        false => eshelf_d,
    };
    // A parameter only while it is used; one an edit stopped using goes to 0, so that the
    // next edit does not take it up again.
    let edh = (input.drawer_h > 0.0 || kit.has("drawer_height"))
        .then(|| parameter("drawer_height", input.drawer_h, "drawer front height"))
        .filter(|_| input.drawer_h > 0.0);
    // The fronts from the plinth (or the bottom panel) up to the top (or under the top
    // panel). With doors the drawers take `stack` of that at the top or the bottom.
    let (low, high) = (
        format!("{epl} + {ein} + {er}"),
        format!("{eh} - {ein} - {er}"),
    );
    let door_h = format!("{high} - {epl} - {ein} - {er}");
    let stack = (edh.as_ref())
        .filter(|_| input.mixed)
        .map(|edh| format!("{}*({edh} + {er})", drawers as u32));
    let (edoor, edraw, erail) = match stack {
        None => ([low.clone(), high, door_h], low, None),
        Some(stack) if input.drawers_top => {
            let z1 = format!("{high} - {stack}");
            let rail = format!("{z1} + {er}/2 - {et}/2");
            let door_h = format!("{z1} - {epl} - {ein} - {er}");
            (
                [low, z1.clone(), door_h],
                format!("{z1} + {er}"),
                Some(rail),
            )
        }
        Some(stack) => {
            let z0 = format!("{low} + {stack}");
            let rail = format!("{z0} - {er}/2 - {et}/2");
            ([z0, high, format!("{door_h} - {stack}")], low, Some(rail))
        }
    };
    // The cabinet stands on Z = 0, centred on the origin, so the view orbits around it.
    kit.centre(&ew, &ed);
    // Where a neighbour stands: the two back corners.
    kit.seat("L", false, &mm(0.0), &ed);
    kit.seat("R", false, &ew, &ed);
    let eiw = format!("({ew} - 2*{et})");
    let n = input.dividers as u32;
    let cabinet = Cabinet {
        kit: &kit,
        input,
        joinery,
        ecell: format!(
            "(({eiw} - {}*{et}) / {})",
            n * (1 + u32::from(input.double)),
            n + 1
        ),
        eiw,
        ech: format!("({eh} - {epl} - 2*{et})"),
        ew,
        eh,
        ed,
        et,
        ebt,
        epl,
        eside_d,
        eshelf_d,
        emount,
        er,
        egrooves,
        eframe,
        egap,
        ein,
        efront,
        eback,
        edoor,
        edraw,
        edh,
        erail,
    };
    let pins = cabinet.carcass(); // holes, run after the panels exist
    doors::build(&cabinet);
    drawers::build(&cabinet);
    // the joints before the pin rows: a cut into a board full of pin holes is slow
    let mut drill = cabinet.joinery.finish();
    drill.extend(pins);
    let (commands, mut summary) = kit.finish(drill, [w, h, d]);
    // The app runs at most 256 commands in one run, and the run itself takes some: also the
    // shared hardware parameters and the models' definitions, the first time, and the moves
    // of the models' copies. Too many: the cabinet is made without the models.
    let cost = crate::hardware::cost(&commands);
    if cost > 230 && kit.models != "off" {
        let mut p = p.clone();
        p["hardware_models"] = json!("off");
        let (commands, mut summary) = build(&p)?;
        summary["hardware_models"] = json!("left out: too many parts for one run");
        return Ok((commands, summary));
    }
    check(
        cost <= 230,
        "too many parts for one run: fewer doors, drawers or shelves, or no labels",
    )?;
    // one compartment
    summary["inner"] =
        json!({ "width": input.cell_w, "height": input.clear_h, "depth": input.shelf_d });
    Ok((commands, summary))
}

impl Cabinet<'_> {
    /// The panels, dividers, shelves, plinth and back. Returns the shelf pin holes.
    fn carcass(&self) -> Commands {
        let Cabinet {
            kit,
            input,
            ew,
            eh,
            ed,
            et,
            ebt,
            epl,
            eiw,
            ech,
            eside_d,
            eshelf_d,
            ein,
            ecell,
            erail,
            emount,
            ..
        } = self;
        let name = kit.name.as_str();
        // Derived sizes, as expressions.
        let n = input.dividers as u32;
        let s = input.shelves as u32;
        let bottom_top = format!("{epl} + {et}");
        let (ebase, ezone) = self.shelf_zone();
        let egap = format!("(({ezone} - {s}*{et}) / {})", s + 1); // between shelves
        let elast = fittings::top_pin_row(&ezone);

        // A sink base has two rails at the top instead of a top: room for the sink and its pipes.
        let sink = input.kind == "sink";
        let carcass: Vec<String> = ["Side L", "Side R", "Bottom", "Top"]
            .iter()
            .filter(|part| !(sink && **part == "Top"))
            .map(|part| format!("{name} {part}"))
            .collect();
        // A panel, or with `cut` its volume taken out of the carcass panels.
        let panel = |part: &str, corner: [&str; 3], size: [&str; 3], cut: bool| {
            kit.panel(part, corner, size, cut.then_some(&carcass[..]))
        };

        let zero = mm(0.0);
        let plinth = match input.plinth > 0.0 {
            true => input.plinth_type,
            false => "none",
        };
        // On legs or casters the whole carcass stands on them; else the sides reach the floor.
        let (side_z, side_h) = match plinth {
            "legs" | "casters" => (epl.clone(), format!("{eh} - {epl}")),
            _ => (zero.clone(), eh.clone()),
        };
        let right_x = format!("{ew} - {et}");
        for (side, x) in [("Side L", &zero), ("Side R", &right_x)] {
            panel(side, [x, &zero, &side_z], [et, eside_d, &side_h], false);
        }
        // In a dado the bottom and top (and a sink base's rails) reach into the sides.
        let (end_x, end_w) = match self.joinery.extension(CARCASS) {
            Some(g) => (format!("{et} - {g}"), format!("{eiw} + 2*{g}")),
            None => (et.clone(), eiw.clone()),
        };
        let (et_, eiw) = (&end_x, &end_w);
        let (bottom_x, bottom_w) = match self.joinery.extension(BOTTOM) {
            Some(g) => (format!("{et} - {g}"), format!("{} + 2*{g}", self.eiw)),
            None => (et.clone(), self.eiw.clone()),
        };
        panel(
            "Bottom",
            [&bottom_x, &zero, epl],
            [&bottom_w, eside_d, et],
            false,
        );
        let top_z = format!("{eh} - {et}");
        match sink {
            // In front of the back: the shelves' depth ends there, also behind inset fronts.
            true => {
                let rail_d = mm(RAIL_D);
                let back_y = format!("{ein} + {eshelf_d} - {rail_d}");
                panel(
                    "Rail front",
                    [et_, &zero, &top_z],
                    [eiw, &rail_d, et],
                    false,
                );
                panel(
                    "Rail back",
                    [et_, &back_y, &top_z],
                    [eiw, &rail_d, et],
                    false,
                );
            }
            false => panel("Top", [et_, &zero, &top_z], [eiw, eside_d, et], false),
        }
        let eiw = &self.eiw;
        self.carcass_joints();
        // The mount board across the back under the top, its rear face flush with the sides'.
        // Box edges: front bottom 9, back top 3.
        if let Some([tm, mh]) = emount {
            let y = format!("{ed} - {tm}");
            let z = format!("{top_z} - {mh}");
            let board = |part: &str, at: [&str; 3], size: [&str; 3]| panel(part, at, size, false);
            let at = [[et.as_str(), &y, &z], [eiw.as_str(), tm, mh]];
            fittings::mount_board(kit, input.wall_mount, tm, board, at, "", [9, 3]);
        }
        // The dividers' boards. They stand between bottom and top, as deep as the shelves.
        let dividers = self.dividers();
        for (part, x, _) in &dividers {
            let (z0, tall) = match self.joinery.extension(INNER) {
                Some(g) => (format!("{bottom_top} - {g}"), format!("{ech} + 2*{g}")),
                None => (bottom_top.clone(), ech.to_string()),
            };
            panel(part, [x, ein, &z0], [et, eshelf_d, &tall], false);
            // its ends into the bottom's top face and the top's underside
            let mid = format!("{x} + {et}/2");
            for (face, z, up) in [
                ("Bottom", bottom_top.clone(), 1),
                ("Top", top_z.clone(), -1),
            ] {
                let joint = Joint {
                    face: face.into(),
                    end: part.clone(),
                    at: [mid.clone(), ein.clone(), z],
                    into: [0, 0, up],
                    out: [-1, 0, 0],
                    along: [0, 1, 0],
                    run: eshelf_d.clone(),
                    single: false,
                    between: false,
                };
                self.joinery.joint(INNER, &joint);
            }
        }
        // The same shelves in every compartment.
        for cell in 0..=n {
            for i in 0..s {
                // Height over the bottom panel: evenly spaced, or on the nearest row of pins.
                let mut over = format!("{egap}*{} + {et}*{i}", i + 1);
                if input.pins {
                    over = fittings::on_pin_row(&over, &elast);
                }
                let part = match n {
                    0 => format!("Shelf {}", i + 1),
                    _ => format!("Shelf {}.{}", cell + 1, i + 1), // compartment.shelf
                };
                let x = self.cell_x(cell);
                let z = format!("{ebase} + {over}");
                if !input.fixed {
                    panel(&part, [&x, ein, &z], [ecell, eshelf_d, et], false);
                    if input.pilasters {
                        self.clips(&part, &x, &z);
                    }
                    continue;
                }
                // Fixed: joined at both ends, to a side or a divider, by the inside's method;
                // in a dado as long again as the grooves.
                let (x0, wide) = match self.joinery.extension(INNER) {
                    Some(g) => (format!("{x} - {g}"), format!("{ecell} + 2*{g}")),
                    None => (x.clone(), ecell.clone()),
                };
                panel(&part, [&x0, ein, &z], [&wide, eshelf_d, et], false);
                // the compartment's left end, then its right
                let face = |k: u32, right: bool| match (k, input.double) {
                    (0, _) => "Side L".to_string(),
                    (k, _) if k > n => "Side R".to_string(),
                    (k, false) => format!("Divider {k}"),
                    (k, true) => format!("Divider {k} {}", if right { "L" } else { "R" }),
                };
                let mid = format!("{z} + {et}/2");
                let right = format!("{x} + {ecell}");
                for (face, plane, into) in [
                    (face(cell, false), &x, 1),
                    (face(cell + 1, true), &right, -1),
                ] {
                    let joint = Joint {
                        face,
                        end: part.clone(),
                        at: [plane.clone(), ein.clone(), mid.clone()],
                        into: [into, 0, 0],
                        out: [0, 0, -1],
                        along: [0, 1, 0],
                        run: eshelf_d.clone(),
                        single: false,
                        // right of a divider: between the holes from the shelf on its left
                        // (a double divider's two boards each carry one shelf)
                        between: into == 1 && cell > 0 && !input.double,
                    };
                    self.joinery.joint(INNER, &joint);
                }
            }
        }
        if let Some(rail) = erail {
            let (x0, wide) = match self.joinery.extension(INNER) {
                Some(g) => (format!("{et} - {g}"), format!("{eiw} + 2*{g}")),
                None => (et.clone(), eiw.clone()),
            };
            panel("Rail", [&x0, ein, rail], [&wide, eshelf_d, et], false);
            for (side, x, into) in [("Side L", et, 1), ("Side R", &right_x, -1)] {
                let joint = Joint {
                    face: side.into(),
                    end: "Rail".into(),
                    at: [x.clone(), ein.clone(), format!("{rail} + {et}/2")],
                    into: [into, 0, 0],
                    out: [0, 0, 1],
                    along: [0, 1, 0],
                    run: eshelf_d.clone(),
                    single: false,
                    between: false,
                };
                self.joinery.joint(INNER, &joint);
            }
        }
        // A parameter while it is used, or once made (as the reveal): an edit takes it up.
        let kick = match matches!(plinth, "board" | "notch") || kit.has("plinth_setback") {
            true => kit.parameter("plinth_setback", input.plinth_setback, "plinth set-back"),
            false => mm(input.plinth_setback),
        };
        match plinth {
            "board" => panel("Plinth", [et, &kick, &zero], [eiw, et, epl], false),
            // The toe kick cut out of each side's front bottom corner: a board across a whole
            // run goes there later. Named `<Side> notch`, which the cut list notes.
            "notch" => {
                for (side, x) in [("Side L", &zero), ("Side R", &right_x)] {
                    let target = [format!("{name} {side}")];
                    let part = format!("{side} notch");
                    kit.panel(&part, [x, &zero, &zero], [et, &kick, epl], Some(&target));
                }
            }
            "legs" | "casters" => {
                // A caster's plate stays under the bottom panel, clear of the sides.
                let inset = match plinth {
                    "legs" => mm(LEG_INSET),
                    _ => format!("({et} + {})", mm(LEG_INSET)),
                };
                let mut xs = vec![inset.clone(), format!("{ew} - {inset}")];
                // ponytail: the count is fixed when it is built, as the shelves' is.
                if input.w > 900.0 {
                    xs.push(format!("{ew}/2"));
                }
                let inset = mm(LEG_INSET);
                let ys = [inset.clone(), format!("{eside_d} - {inset}")];
                let at = xs.iter().flat_map(|x| ys.iter().map(move |y| (x, y)));
                for (n, (x, y)) in at.enumerate() {
                    fittings::foot(kit, plinth, n + 1, [x, y], epl, [et, "Bottom"]);
                }
            }
            _ => {}
        }
        match input.back {
            "overlay" => {
                let y = format!("{ed} - {ebt}");
                panel(
                    "Back",
                    [&zero, &y, epl],
                    [ew, ebt, &format!("{eh} - {epl}")],
                    false,
                )
            }
            "groove" => {
                // Cut the back panel's own volume out of the carcass first: stopped grooves
                // that fit exactly. In front of a mount board, by its thickness.
                let inset = emount.as_ref().map_or(mm(GROOVE_INSET), |m| m[0].clone());
                let g = format!("min({et}/2, {})", mm(10.0)); // groove depth
                let corner = [
                    format!("{et} - {g}"),
                    format!("{ed} - {inset} - {ebt}"),
                    format!("{bottom_top} - {g}"),
                ];
                let corner = [corner[0].as_str(), &corner[1], &corner[2]];
                let size = [
                    format!("{eiw} + 2*{g}"),
                    ebt.clone(),
                    format!("{ech} + 2*{g}"),
                ];
                let size = [size[0].as_str(), &size[1], &size[2]];
                panel("Back groove", corner, size, true);
                panel("Back", corner, size, false);
            }
            _ => {}
        }
        match (input.pins, input.pilasters) {
            (true, true) => self.strips(&dividers, &ebase, &ezone),
            (true, false) => self.pins(&dividers, &elast, &ebase),
            _ => Vec::new(),
        }
    }

    /// The sides to the bottom and the top, or a sink base's rails.
    fn carcass_joints(&self) {
        let Cabinet {
            input,
            ew,
            eh,
            et,
            epl,
            ein,
            eshelf_d,
            eside_d,
            ..
        } = self;
        let zero = mm(0.0);
        let rail_y = format!("{ein} + {eshelf_d} - {}", mm(RAIL_D));
        let mut ends = vec![("Bottom", format!("{epl} + {et}/2"), zero.clone())];
        let top = format!("{eh} - {et}/2");
        match input.kind {
            "sink" => ends.extend([
                ("Rail front", top.clone(), zero),
                ("Rail back", top, rail_y),
            ]),
            _ => ends.push(("Top", top, zero)),
        }
        let right = format!("{ew} - {et}");
        for (side, x, into) in [("Side L", et, 1), ("Side R", &right, -1)] {
            for (end, z, y) in &ends {
                let single = end.starts_with("Rail");
                let joint = Joint {
                    face: side.into(),
                    end: end.to_string(),
                    at: [x.clone(), y.clone(), z.clone()],
                    into: [into, 0, 0],
                    // under the bottom, over the top and the rails
                    out: [0, 0, if *end == "Bottom" { -1 } else { 1 }],
                    along: [0, 1, 0],
                    run: match single {
                        true => mm(RAIL_D),
                        false => eside_d.clone(),
                    },
                    single,
                    between: false,
                };
                let scope = if *end == "Bottom" { BOTTOM } else { CARCASS };
                self.joinery.joint(scope, &joint);
            }
        }
    }

    /// Each divider's boards: its name, its left face and the face its shelves rest on (0 both,
    /// -1 its left, 1 its right). A double divider is two boards side by side, `L` and `R`,
    /// each carrying the shelves on its outer face.
    fn dividers(&self) -> Vec<(String, String, i32)> {
        let Cabinet { input, et, .. } = self;
        let mut boards = Vec::new();
        for k in 1..=input.dividers as u32 {
            let right = self.cell_x(k); // the compartment right of it
            match input.double {
                false => boards.push((format!("Divider {k}"), format!("{right} - {et}"), 0)),
                true => {
                    boards.push((format!("Divider {k} L"), format!("{right} - 2*{et}"), -1));
                    boards.push((format!("Divider {k} R"), format!("{right} - {et}"), 1));
                }
            }
        }
        boards
    }

    /// Pilaster strips (#264) instead of pin rows: in each face that carries shelves, a dado
    /// `STRIP` wide and deep on the pin rows' front and back lines over the shelves' height,
    /// and a strip in it, flush with the face: `<board> pilaster dado <row>` and the model
    /// `<board> pilaster <row> (<length> mm)` (a name with ` pilaster ` is a hardware model,
    /// which the cut list counts by that length). A divider carrying shelves on both faces has a
    /// pair on each (`left`, `right`).
    fn strips(&self, dividers: &[(String, String, i32)], base: &str, zone: &str) -> Commands {
        let Cabinet {
            kit,
            input,
            ew,
            et,
            ein,
            eshelf_d,
            ..
        } = self;
        // the face's x, the way into the board, and what tells two faces of one board apart
        let mut faces = vec![
            ("Side L".to_string(), et.clone(), -1, ""),
            ("Side R".to_string(), format!("{ew} - {et}"), 1, ""),
        ];
        for (part, x, face) in dividers {
            match face {
                0 => {
                    faces.push((part.clone(), x.clone(), 1, " left"));
                    faces.push((part.clone(), format!("{x} + {et}"), -1, " right"));
                }
                -1 => faces.push((part.clone(), x.clone(), 1, "")),
                _ => faces.push((part.clone(), format!("{x} + {et}"), -1, "")),
            }
        }
        let [w, deep] = [mm(STRIP[0]), mm(STRIP[1])];
        let length = (input.clear_h).round();
        let rows = [
            ("front", format!("{ein} + {}", mm(PIN_SETBACK))),
            ("back", format!("{ein} + {eshelf_d} - {}", mm(PIN_SETBACK))),
        ];
        for (board, fx, into, which) in faces {
            let x0 = match into {
                1 => fx.clone(),
                _ => format!("{fx} - {deep}"),
            };
            let target = [format!("{} {board}", kit.name)];
            for (row, y) in &rows {
                let y0 = format!("{y} - {w}/2");
                let (corner, size) = ([x0.as_str(), &y0, base], [deep.as_str(), &w, zone]);
                kit.panel(
                    &format!("{board} pilaster dado {row}{which}"),
                    corner,
                    size,
                    Some(&target),
                );
                let strip = format!("{board} pilaster {row}{which} ({length} mm)");
                kit.model(&strip, corner, size);
            }
        }
        // ponytail: made with the carcass, not with the holes after the joinery
        Vec::new()
    }

    /// Four clips under a shelf on pilaster strips (#264): one model `<shelf> clip` at its
    /// front left, out of the strip's face, and a pattern of it to the other three.
    fn clips(&self, shelf: &str, x: &str, z: &str) {
        let Cabinet {
            kit,
            ein,
            ecell,
            eshelf_d,
            ..
        } = self;
        let [w, out, high] = [mm(STRIP[0]), mm(CLIP[0]), mm(CLIP[1])];
        let clip = format!("{shelf} clip");
        let y = format!("{ein} + {} - {w}/2", mm(PIN_SETBACK));
        kit.model(&clip, [x, &y, &format!("{z} - {high}")], [&out, &w, &high]);
        kit.push((
            "solid.pattern.rectangular",
            json!({
                "bodies": [format!("{} {clip}", kit.name)], "name": format!("{} {clip}s", kit.name),
                "dir1": [1, 0, 0], "count1": 2, "spacing1": format!("{ecell} - {out}"),
                "dir2": [0, 1, 0], "count2": 2, "spacing2": format!("{eshelf_d} - {}", mm(2.0 * PIN_SETBACK)),
            }),
        ));
    }

    /// One hole per panel, repeated up the panel and across to the rear row. Blind in the
    /// sides and a double divider's boards; through a divider, which carries shelves on both
    /// faces.
    fn pins(&self, dividers: &[(String, String, i32)], top_row: &str, base: &str) -> Commands {
        let Cabinet {
            kit, ew, et, ein, ..
        } = self;
        let blind = fittings::blind_pin(et);
        let mut rows = vec![
            ("Side L".to_string(), et.clone(), blind.clone(), -1),
            (
                "Side R".to_string(),
                format!("{ew} - {et}"),
                blind.clone(),
                1,
            ),
        ];
        for (part, x, face) in dividers {
            rows.push(match face {
                0 => (part.clone(), x.clone(), et.clone(), 1),
                -1 => (part.clone(), x.clone(), blind.clone(), 1),
                _ => (part.clone(), format!("{x} + {et}"), blind.clone(), -1),
            });
        }
        let z = format!("{base} + {}", mm(2.0 * PITCH));
        let mut drill = Vec::new();
        for (part, x, depth, direction) in rows {
            let rows = PinRows {
                part: &part,
                position: kit.at(&x, &format!("{ein} + {}", mm(PIN_SETBACK)), &z),
                into: [direction, 0, 0],
                depth: &depth,
                across: [0, 1, 0],
                run: &self.eshelf_d,
            };
            drill.extend(fittings::pin_rows(kit, top_row, rows));
        }
        fittings::shared_pin_rows(kit, drill)
    }

    /// Where the shelves' part starts (the bottom panel's top face, the base of the pin rows'
    /// grid), and its clear height: with doors and drawers, the doors' part on the other side
    /// of the rail.
    fn shelf_zone(&self) -> (String, String) {
        let Cabinet {
            input,
            eh,
            et,
            epl,
            ech,
            erail,
            ..
        } = self;
        let bottom_top = format!("{epl} + {et}");
        match erail {
            Some(rail) if input.drawers_top => (bottom_top, format!("({rail} - {epl} - {et})")),
            Some(rail) => (
                format!("{rail} + {et}"),
                format!("({eh} - 2*{et} - ({rail}))"),
            ),
            None => (bottom_top, ech.clone()),
        }
    }

    /// The left face of compartment `cell`.
    fn cell_x(&self, cell: u32) -> String {
        let Cabinet {
            input, et, ecell, ..
        } = self;
        match input.double {
            false => format!("{et} + ({ecell} + {et}) * {cell}"),
            true => format!("{et} + ({ecell} + 2*{et}) * {cell}"),
        }
    }

    /// A grip edge or finger groove along a front's top or bottom edge (`fittings::grip`): the
    /// front is the box from `lo` to `lo + size`, its back face towards +y.
    fn grip(&self, body: &str, lo: [&str; 3], size: [&str; 3], top: bool) {
        let at = |x: &str, y: &str, z: &str| self.kit.at(x, y, z);
        fittings::grip(self.kit, body, self.handles(body), at, lo, size, 0, 1, top);
    }

    /// A door or drawer front (`fittings::front`): one board, or a shaker front when the
    /// cabinet has them and `framed`; with glass fronts a pane instead of the panel. Its back face
    /// towards +y; `main_low`: `<body>` is the left stile.
    fn front(&self, body: &str, lo: [&str; 3], size: [&str; 3], main_low: bool, framed: bool) {
        let at = |x: &str, y: &str, z: &str| self.kit.at(x, y, z);
        let frame = self.eframe.as_ref().filter(|_| framed);
        let glass = self.input.glass;
        fittings::front(self.kit, body, frame, at, lo, size, 0, main_low, glass);
    }

    /// A front from `z`, `h` high, cut back by the grip gap at its grip edge (`top`, else the
    /// bottom) when it has a grip edge (`fittings::gripped`).
    fn gripped(&self, z: &str, h: &str, top: bool) -> (String, String) {
        let gap = self
            .egap
            .as_deref()
            .filter(|_| self.input.handles == "grip");
        fittings::gripped(gap, z, h, top)
    }

    /// `gripped` for a door, by the doors' handles.
    fn gripped_door(&self, z: &str, h: &str, top: bool) -> (String, String) {
        let gap = self
            .egap
            .as_deref()
            .filter(|_| self.input.door_handles == "grip");
        fittings::gripped(gap, z, h, top)
    }

    /// The handles of a front by its part: a door's are the doors' (`Input::door_handles`).
    fn handles(&self, body: &str) -> &str {
        match body.starts_with("Door") {
            true => self.input.door_handles,
            false => self.input.handles,
        }
    }

    /// Grooves in a front's face (`fittings::front_grooves`), the front as for `grip`.
    fn grooves(&self, body: &str, lo: [&str; 3], size: [&str; 3]) {
        if let (Some(g), Some(sizes)) = (&self.input.grooves, &self.egrooves) {
            let at = |x: &str, y: &str, z: &str| self.kit.at(x, y, z);
            fittings::front_grooves(self.kit, body, g, sizes, at, lo, size, 0, 1);
        }
    }

    /// Screw holes for a knob (one) or a pull (two), through a front from its back face.
    /// `along` is the axis the pull lies on: 0 across, 2 upright.
    fn handle(&self, body: &str, x: &str, z: &str, along: usize) {
        let offsets = fittings::screw_offsets(self.handles(body));
        for (j, offset) in offsets.iter().enumerate() {
            let mut at = [x.to_string(), self.eback.clone(), z.to_string()];
            at[along] = format!("{} + {}", at[along], mm(*offset));
            let position = self.kit.at(&at[0], &at[1], &at[2]);
            let what = format!("handle {}", j + 1);
            fittings::hole(
                self.kit,
                body,
                position,
                [0, -1, 0],
                &mm(HANDLE_D),
                &self.et,
                &what,
            );
        }
    }
}

#[cfg(test)]
mod tests;

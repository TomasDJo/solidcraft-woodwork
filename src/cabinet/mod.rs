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

use crate::fittings::{self, PinRows, HANDLE_D, LEG_INSET, PIN_SETBACK, PITCH};
use crate::joint::{Joinery, Joint, CARCASS, INNER};
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
    // Dividers and the rail between drawers and doors take the second method.
    let inner = input.dividers > 0.0 || input.mixed;
    let joinery = Joinery::new(&kit, p, (input.t, &et), inner)?;
    let side_mm = if back == "overlay" { d - input.bt } else { d };
    joinery.fits(CARCASS, side_mm, p, "the sides")?;
    joinery.fits(INNER, input.shelf_d, p, "the dividers and the rail")?;
    let ebt = match back {
        "none" => mm(0.0),
        _ => parameter("back_thickness", input.bt, "back panel thickness"),
    };
    let epl = match input.plinth > 0.0 {
        true => parameter("plinth", input.plinth, "plinth height"),
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
        ecell: format!("(({eiw} - {n}*{et}) / {})", n + 1),
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
    // shared hardware parameters, the first time.
    let shared = match crate::hardware::used(&commands) {
        true => crate::hardware::PARAMETERS.len(),
        false => 0,
    };
    check(
        commands.len() + shared <= 230,
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
        panel("Bottom", [et, &zero, epl], [eiw, eside_d, et], false);
        let top_z = format!("{eh} - {et}");
        match sink {
            // In front of the back: the shelves' depth ends there, also behind inset fronts.
            true => {
                let rail_d = mm(RAIL_D);
                let back_y = format!("{ein} + {eshelf_d} - {rail_d}");
                panel("Rail front", [et, &zero, &top_z], [eiw, &rail_d, et], false);
                panel(
                    "Rail back",
                    [et, &back_y, &top_z],
                    [eiw, &rail_d, et],
                    false,
                );
            }
            false => panel("Top", [et, &zero, &top_z], [eiw, eside_d, et], false),
        }
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
        // Left face of each divider. They stand between bottom and top, as deep as the shelves.
        let divider_x: Vec<String> = (1..=n).map(|k| format!("({ecell} + {et}) * {k}")).collect();
        for (k, x) in divider_x.iter().enumerate() {
            let part = format!("Divider {}", k + 1);
            panel(&part, [x, ein, &bottom_top], [et, eshelf_d, ech], false);
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
                    along: [0, 1, 0],
                    run: eshelf_d.clone(),
                    single: false,
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
                panel(&part, [&x, ein, &z], [ecell, eshelf_d, et], false);
            }
        }
        if let Some(rail) = erail {
            panel("Rail", [et, ein, rail], [eiw, eshelf_d, et], false);
            for (side, x, into) in [("Side L", et, 1), ("Side R", &right_x, -1)] {
                let joint = Joint {
                    face: side.into(),
                    end: "Rail".into(),
                    at: [x.clone(), ein.clone(), format!("{rail} + {et}/2")],
                    into: [into, 0, 0],
                    along: [0, 1, 0],
                    run: eshelf_d.clone(),
                    single: false,
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
        match input.pins {
            true => self.pins(&divider_x, &elast, &ebase),
            false => Vec::new(),
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
                    along: [0, 1, 0],
                    run: match single {
                        true => mm(RAIL_D),
                        false => eside_d.clone(),
                    },
                    single,
                };
                self.joinery.joint(CARCASS, &joint);
            }
        }
    }

    /// One hole per panel, repeated up the panel and across to the rear row. Blind in the
    /// sides; through a divider, which carries shelves on both faces.
    fn pins(&self, divider_x: &[String], top_row: &str, base: &str) -> Commands {
        let Cabinet {
            kit, ew, et, ein, ..
        } = self;
        let blind = fittings::blind_pin(et);
        let mut rows = vec![
            ("Side L".to_string(), et.clone(), blind.clone(), -1),
            ("Side R".to_string(), format!("{ew} - {et}"), blind, 1),
        ];
        for (k, x) in divider_x.iter().enumerate() {
            rows.push((format!("Divider {}", k + 1), x.clone(), et.clone(), 1));
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
        drill
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
        let Cabinet { et, ecell, .. } = self;
        format!("{et} + ({ecell} + {et}) * {cell}")
    }

    /// Screw holes for a knob (one) or a pull (two), through a front from its back face.
    /// `along` is the axis the pull lies on: 0 across, 2 upright.
    fn handle(&self, body: &str, x: &str, z: &str, along: usize) {
        let offsets = fittings::screw_offsets(self.input.handles);
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

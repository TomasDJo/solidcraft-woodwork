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

use crate::fittings::{self, PinRows, HANDLE_D, PIN_SETBACK, PITCH};
use crate::kit::{mm, Kit};
use crate::Commands;
use input::Input;
use serde_json::{json, Value};

const GROOVE_INSET: f64 = 12.0; // back groove, from the rear edge
const KICK_RECESS: f64 = 50.0; // plinth board, back from the front

/// What the steps of a build share: the checked input, and the sizes as expressions.
struct Cabinet<'a> {
    kit: &'a Kit,
    input: Input<'a>,
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
    // The reveal around the fronts; how far the fronts' opening is inside the carcass's
    // outside (a panel when inset, which is also where the shelves start); and the y of the
    // fronts' front and back faces.
    er: String,
    ein: String,
    efront: String,
    eback: String,
    ecell: String, // one compartment's width
    // The doors' top edge, and the top drawer's front height when it is a parameter.
    ez1: String,
    edh: Option<String>,
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
        drawer_h,
        ..
    } = input;

    let parameter = |key: &str, mm_value: f64, comment: &str| kit.parameter(key, mm_value, comment);
    let ew = parameter("width", w, "outside width");
    let eh = parameter("height", h, "outside height, plinth included");
    let ed = parameter("depth", d, "outside depth, back included");
    let et = parameter("thickness", input.t, "panel thickness");
    let ebt = match back {
        "none" => mm(0.0),
        _ => parameter("back_thickness", input.bt, "back panel thickness"),
    };
    let epl = match input.plinth > 0.0 {
        true => parameter("plinth", input.plinth, "plinth height"),
        false => mm(0.0),
    };
    let (eside_d, eshelf_d) = match back {
        "overlay" => (format!("({ed} - {ebt})"), format!("({ed} - {ebt})")),
        "groove" => (ed.clone(), format!("({ed} - {} - {ebt})", mm(GROOVE_INSET))),
        _ => (ed.clone(), ed.clone()),
    };
    // A parameter only while it is used; one an edit stopped using goes to 0, so that the
    // next edit does not take it up again.
    let edh = (drawer_h > 0.0 || kit.has("drawer_height"))
        .then(|| parameter("drawer_height", drawer_h, "drawer front height"))
        .filter(|_| drawer_h > 0.0);
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
    let ez1 = match (drawers > 0.0, doors > 0.0, &edh) {
        (false, ..) => format!("{eh} - {ein} - {er}"),
        // inset: the rail and a gap under it too
        (true, true, Some(edh)) if input.inset => {
            format!("({eh} - {et} - {er} - {drawers}*({edh} + {er}) - {et} - {er})")
        }
        (true, true, Some(edh)) => format!("({eh} - {er} - {drawers}*({edh} + {er}))"),
        _ => format!("{epl} + {ein}"), // all drawers
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
        er,
        ein,
        efront,
        eback,
        ez1,
        edh,
    };
    let drill = cabinet.carcass(); // holes, run after the panels exist
    doors::build(&cabinet);
    drawers::build(&cabinet);
    let (commands, mut summary) = kit.finish(drill, [w, h, d]);
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
            er,
            ein,
            ecell,
            ez1,
            ..
        } = self;
        let name = kit.name.as_str();
        // Derived sizes, as expressions.
        let n = input.dividers as u32;
        let s = input.shelves as u32;

        // Drawers over doors: a rail between the sides, behind the gap under the lowest drawer
        // front, or with inset fronts flush with the front between the drawers and the doors.
        // The shelves are in the doors' part below it.
        let rail = input.drawers > 0.0 && input.doors > 0.0;
        let rail_z = match input.inset {
            true => format!("{ez1} + {er}"),
            false => format!("{ez1} + {er}/2 - {et}/2"),
        };
        let ezone = match rail {
            true => format!("({rail_z} - {epl} - {et})"),
            false => ech.clone(),
        };
        let egap = format!("(({ezone} - {s}*{et}) / {})", s + 1); // between shelves
        let elast = fittings::top_pin_row(&ezone);

        let carcass = ["Side L", "Side R", "Bottom", "Top"].map(|part| format!("{name} {part}"));
        // A panel, or with `cut` its volume taken out of the carcass panels.
        let panel = |part: &str, corner: [&str; 3], size: [&str; 3], cut: bool| {
            kit.panel(part, corner, size, cut.then_some(&carcass[..]))
        };

        let zero = mm(0.0);
        let bottom_top = format!("{epl} + {et}"); // top face of the bottom panel
        panel("Side L", [&zero, &zero, &zero], [et, eside_d, eh], false);
        panel(
            "Side R",
            [&format!("{ew} - {et}"), &zero, &zero],
            [et, eside_d, eh],
            false,
        );
        panel("Bottom", [et, &zero, epl], [eiw, eside_d, et], false);
        panel(
            "Top",
            [et, &zero, &format!("{eh} - {et}")],
            [eiw, eside_d, et],
            false,
        );
        // Left face of each divider. They stand between bottom and top, as deep as the shelves.
        let divider_x: Vec<String> = (1..=n).map(|k| format!("({ecell} + {et}) * {k}")).collect();
        for (k, x) in divider_x.iter().enumerate() {
            let part = format!("Divider {}", k + 1);
            panel(&part, [x, ein, &bottom_top], [et, eshelf_d, ech], false);
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
                let z = format!("{bottom_top} + {over}");
                panel(&part, [&x, ein, &z], [ecell, eshelf_d, et], false);
            }
        }
        if rail {
            let depth = format!("{eshelf_d} + {ein}");
            panel("Rail", [et, &zero, &rail_z], [eiw, &depth, et], false);
        }
        if input.plinth > 0.0 {
            panel(
                "Plinth",
                [et, &mm(KICK_RECESS), &zero],
                [eiw, et, epl],
                false,
            );
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
                // that fit exactly.
                let g = format!("min({et}/2, {})", mm(10.0)); // groove depth
                let corner = [
                    format!("{et} - {g}"),
                    format!("{ed} - {} - {ebt}", mm(GROOVE_INSET)),
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
            true => self.pins(&divider_x, &elast),
            false => Vec::new(),
        }
    }

    /// One hole per panel, repeated up the panel and across to the rear row. Blind in the
    /// sides; through a divider, which carries shelves on both faces.
    fn pins(&self, divider_x: &[String], top_row: &str) -> Commands {
        let Cabinet {
            kit,
            ew,
            et,
            epl,
            ein,
            ..
        } = self;
        let blind = fittings::blind_pin(et);
        let mut rows = vec![
            ("Side L".to_string(), et.clone(), blind.clone(), -1),
            ("Side R".to_string(), format!("{ew} - {et}"), blind, 1),
        ];
        for (k, x) in divider_x.iter().enumerate() {
            rows.push((format!("Divider {}", k + 1), x.clone(), et.clone(), 1));
        }
        let z = format!("{epl} + {et} + {}", mm(2.0 * PITCH));
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
                HANDLE_D,
                &self.et,
                &what,
            );
        }
    }
}

#[cfg(test)]
mod tests;

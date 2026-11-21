//! L-shaped corner cabinet: one leg along the back wall, the other (the return) running
//! forward along a side wall.
//!
//! Written for a corner at the back left: the footprint's bounding box is `width` (X) by
//! `return` (Y, the front at 0), and the return is on the left. A right corner is the same
//! mirrored in X. Like the box cabinet, every size is an expression over design parameters.
//!
//! The diagonal shape has one front straight across the open corner, at 45 degrees, in place
//! of the two fronts of the L.
//!
//! This file is the carcass; `input.rs` reads the dialog, `doors.rs` is the L's bi-fold
//! pair, `diagonal.rs` the diagonal shape's door and plinth.

mod diagonal;
mod doors;
mod input;

use crate::fittings::{self, PinRows, LEG_INSET, PIN_SETBACK, PITCH};
use crate::kit::{mm, Kit};
use crate::Commands;
pub use input::hand;
use input::Input;
use serde_json::{json, Value};

/// What the steps of a build share: the checked input in mm, and the sizes as expressions.
struct Corner<'a> {
    kit: &'a Kit,
    right: bool,
    diagonal: bool,
    handles: &'a str,
    handle_at: &'a str,
    a: f64,
    b: f64,
    d: f64,
    h: f64,
    t: f64,
    plinth: f64,
    plinth_type: &'a str, // board, notch or legs: see `fittings::plinth_type`
    dr: f64,              // depth at the return's end: `d` on the L
    g: f64,               // the reveal around the doors, and its parameter
    inset: bool,          // the doors inside the opening, the shelves a panel back behind them
    er: String,
    ea: String,
    eb: String,
    ed: String,
    edr: String,
    eh: String,
    et: String,
    epl: String,
    ekick: String, // the plinth board's or the notch's set-back
}

pub fn build(p: &Value) -> Result<(Commands, Value), String> {
    let kit = Kit::new(p)?;
    let Input {
        a,
        b,
        d,
        h,
        t,
        plinth,
        plinth_type,
        plinth_setback,
        dr,
        shelves,
        pins,
        on_return,
        handles,
        handle_at,
        right,
        diagonal,
        reveal,
        inset,
    } = input::read(&kit, p)?;

    let ea = kit.parameter("width", a, "along the back wall");
    let eb = kit.parameter("return", b, "along the side wall");
    let ed = kit.parameter("depth", d, "depth of both legs");
    let eh = kit.parameter("height", h, "outside height, plinth included");
    let et = kit.parameter("thickness", t, "panel thickness");
    let epl = match plinth > 0.0 {
        true => kit.parameter("plinth", plinth, "plinth height"),
        false => mm(0.0),
    };
    // As on the box cabinet.
    let ekick = match plinth > 0.0 && plinth_type != "legs" || kit.has("plinth_setback") {
        true => kit.parameter("plinth_setback", plinth_setback, "plinth set-back"),
        false => mm(plinth_setback),
    };
    // Only with doors, or when an edit took them off (as on the box cabinet).
    let er = match on_return.is_some() || kit.has("reveal") {
        true => kit.parameter_as("reveal", reveal, "gap around the doors", mm(reveal)),
        false => mm(reveal),
    };
    kit.centre(&ea, &eb);
    let edr = match diagonal {
        true => format!("({ea} - {eb} + {ed})"),
        false => ed.clone(),
    };
    let corner = Corner {
        kit: &kit,
        right,
        diagonal,
        handles,
        handle_at,
        a,
        b,
        d,
        h,
        t,
        plinth,
        plinth_type: if plinth > 0.0 { plinth_type } else { "none" },
        dr,
        g: reveal,
        inset,
        er,
        ea,
        eb,
        ed,
        edr,
        eh,
        et,
        epl,
        ekick,
    };
    let drill = corner.carcass(shelves, pins); // holes, run after the panels exist
    match on_return {
        Some(on_return) if diagonal => diagonal::door(&corner, on_return),
        Some(on_return) => doors::build(&corner, on_return),
        None => {}
    }
    // The bounding box: `return` is its depth.
    Ok(kit.finish(drill, [a, h, b]))
}

impl Corner<'_> {
    /// Mirrored for a right corner: where a box of this length starts.
    fn from(&self, x: &str, length: &str) -> String {
        match self.right {
            true => format!("{} - ({x}) - ({length})", self.ea),
            false => x.to_string(),
        }
    }

    /// Mirrored for a right corner: where a point is.
    fn point(&self, x: &str) -> String {
        self.from(x, "0 mm")
    }

    fn flip(&self) -> i32 {
        if self.right {
            -1
        } else {
            1
        }
    }

    fn panel(&self, part: &str, corner: [&str; 3], size: [&str; 3], cut: Option<&[String]>) {
        let x = self.from(corner[0], size[0]);
        self.kit.panel(part, [&x, corner[1], corner[2]], size, cut)
    }

    /// The end of the back leg and the end of the return are the cabinet's two sides: together
    /// they span its whole footprint, which is what placing against it measures.
    fn ends(&self) -> (&'static str, &'static str) {
        match self.right {
            true => ("Side L", "Side R"),
            false => ("Side R", "Side L"),
        }
    }

    /// The sides, backs, L-shaped boards and plinth. Returns the shelf pin holes.
    fn carcass(&self, s: u32, pins: bool) -> Commands {
        let Corner {
            ea,
            eb,
            ed,
            edr,
            eh,
            et,
            epl,
            ..
        } = self;
        let name = self.kit.name.as_str();
        let panel = |part: &str, corner: [&str; 3], size: [&str; 3], cut: Option<&[String]>| {
            self.panel(part, corner, size, cut)
        };
        let (back_end, return_end) = self.ends();
        let zero = mm(0.0);
        // Where a neighbour stands: at the end of the back leg its back is on the same wall;
        // at the end of the return the wall is beside it.
        let side = |end: &'static str| &end[end.len() - 1..];
        (self.kit).seat(side(back_end), false, &self.point(ea), eb);
        (self.kit).seat(side(return_end), true, &self.point(&zero), &zero);
        // On legs the whole carcass stands on them; else the ends and backs reach the floor.
        let (z, high) = match self.plinth_type {
            "legs" => (epl.clone(), format!("{eh} - {epl}")),
            _ => (zero.clone(), eh.to_string()),
        };
        let back_end_x = format!("{ea} - {et}");
        let front_y = format!("{eb} - {ed}");
        panel(back_end, [&back_end_x, &front_y, &z], [et, ed, &high], None);
        panel(return_end, [&zero, &zero, &z], [edr, et, &high], None);
        // Full panels against both walls, standing on the floor like the sides.
        panel(
            "Back",
            [et, &format!("{eb} - {et}"), &z],
            [&format!("{ea} - 2*{et}"), et, &high],
            None,
        );
        panel(
            "Back return",
            [&zero, et, &z],
            [et, &format!("{eb} - {et}"), &high],
            None,
        );
        // Bottom, top and shelves are L-shaped: a board over the whole inside, with the open
        // corner cut out of it, square or (diagonal) at 45 degrees. The cut list gives the
        // board before the cut. Each board's height is also kept in mm, for picking its edge.
        let ech = format!("({eh} - {epl} - 2*{et})"); // clear height
        let egap = format!("(({ech} - {s}*{et}) / {})", s + 1); // between shelves
        let (t, clear_h) = (self.t, self.h - self.plinth - 2.0 * self.t);
        let gap = (clear_h - s as f64 * t) / (s + 1) as f64;
        let mut boards = vec![
            ("Bottom".to_string(), epl.clone(), self.plinth),
            ("Top".to_string(), format!("{eh} - {et}"), self.h - t),
        ];
        let elast = fittings::top_pin_row(&ech);
        for i in 0..s {
            // Height over the bottom panel: evenly spaced, or on the nearest row of pins.
            let mut over = format!("{egap}*{} + {et}*{i}", i + 1);
            let mut over_mm = gap * (i + 1) as f64 + t * i as f64;
            if pins {
                over = fittings::on_pin_row(&over, &elast);
                over_mm = fittings::on_pin_row_mm(over_mm, clear_h);
            }
            boards.push((
                format!("Shelf {}", i + 1),
                format!("{epl} + {et} + {over}"),
                self.plinth + t + over_mm,
            ));
        }
        for (part, z, z_mm) in &boards {
            // Behind inset doors a shelf's front edges are a panel further back.
            let back = self.inset && part.starts_with("Shelf");
            panel(
                part,
                [et, et, z],
                [&format!("{ea} - 2*{et}"), &format!("{eb} - 2*{et}"), et],
                None,
            );
            if self.diagonal {
                // The board's upright edge at the open corner, cut back to the front.
                let edge = [
                    (self.a / 2.0 - t) * self.flip() as f64,
                    t - self.b / 2.0,
                    z_mm + t / 2.0,
                ];
                self.kit.push((
                    "solid.chamfer",
                    json!({
                        "edges": [edge], "distance": match back {
                            true => format!("{eb} - {ed} - 2*{et} + sqrt(2)*{et}"),
                            false => format!("{eb} - {ed} - 2*{et}"),
                        },
                        "body": format!("{name} {part}"), "name": format!("{name} {part} corner"),
                    }),
                ));
                continue;
            }
            let notch = match back {
                true => [
                    format!("{ed} - {et}"),
                    format!("{ea} - {ed}"),
                    format!("{eb} - {ed}"),
                ],
                false => [
                    ed.to_string(),
                    format!("{ea} - {et} - {ed}"),
                    format!("{eb} - {ed} - {et}"),
                ],
            };
            panel(
                &format!("{part} notch"),
                [&notch[0], et, z],
                [&notch[1], &notch[2], et],
                Some(&[format!("{name} {part}")]),
            );
        }
        let kick = self.ekick.clone();
        if self.plinth_type == "notch" {
            // The toe kick cut out of each end's front bottom corner, as on the box cabinet.
            let ends = [
                (
                    back_end,
                    [back_end_x.as_str(), &front_y],
                    [et.as_str(), &kick],
                ),
                (return_end, [&format!("{edr} - {kick}"), &zero], [&kick, et]),
            ];
            for (end, [x, y], [sx, sy]) in ends {
                let target = [format!("{name} {end}")];
                panel(
                    &format!("{end} notch"),
                    [x, y, &zero],
                    [sx, sy, epl],
                    Some(&target),
                );
            }
        } else if self.plinth_type == "legs" {
            // Under each corner of the footprint, and the L's inner corner.
            let inset = mm(LEG_INSET);
            let (near, wall_x, wall_y) = (
                inset.clone(),
                format!("{ea} - {inset}"),
                format!("{eb} - {inset}"),
            );
            let mut at = vec![
                (wall_x.clone(), format!("{front_y} + {inset}")),
                (wall_x, wall_y.clone()),
                (near.clone(), wall_y),
                (near.clone(), near.clone()),
                (format!("{edr} - {inset}"), near),
            ];
            if !self.diagonal {
                at.push((format!("{ed} - {inset}"), format!("{front_y} + {inset}")));
            }
            for (n, (x, y)) in at.iter().enumerate() {
                let at = self.kit.at(&self.point(x), y, &zero);
                fittings::leg(self.kit, n + 1, at, epl);
            }
        } else if self.plinth_type == "board" && self.diagonal {
            diagonal::plinth(self);
        } else if self.plinth_type == "board" {
            // Two boards set back from the fronts, meeting in the inner corner.
            let x = format!("{ed} - {kick} - {et}");
            let y = format!("{eb} - {ed} + {kick}");
            panel(
                "Plinth",
                [&x, &y, &zero],
                [&format!("{ea} - {et} - ({x})"), et, epl],
                None,
            );
            panel(
                "Plinth return",
                [&x, et, &zero],
                [et, &format!("{y} - {et}"), epl],
                None,
            );
        }
        match pins {
            true => self.pins(&elast),
            false => Vec::new(),
        }
    }

    /// Like the box cabinet's: one blind hole per panel, repeated up it and across to a
    /// second row. Two rows in each end and in each back, 37 mm from the corners of a leg.
    fn pins(&self, top_row: &str) -> Commands {
        let Corner {
            kit,
            ea,
            eb,
            ed,
            et,
            epl,
            ..
        } = self;
        let flip = self.flip();
        let (back_end, return_end) = self.ends();
        let blind = fittings::blind_pin(et);
        // Behind inset doors the shelves, so the rows, end a panel further back.
        let (front, o) = match self.inset {
            true => (format!("{eb} - {ed} + {et}"), format!(" - {et}")),
            false => (format!("{eb} - {ed}"), String::new()),
        };
        let (near, far) = (
            format!("{et} + {}", mm(PIN_SETBACK)),
            format!("{front} + {}", mm(PIN_SETBACK)),
        );
        let (wall_x, wall_y) = (format!("{ea} - {et}"), format!("{eb} - {et}"));
        // (panel, the first hole on its inner face, the way in, the way to the second row)
        let rows = [
            (back_end, [&wall_x, &far], [flip, 0, 0], [0, 1, 0]),
            (return_end, [&near, et], [0, -1, 0], [flip, 0, 0]),
            ("Back", [&near, &wall_y], [0, 1, 0], [flip, 0, 0]),
            ("Back return", [et, &far], [-flip, 0, 0], [0, 1, 0]),
        ];
        let z = format!("{epl} + {et} + {}", mm(2.0 * PITCH));
        let (run, return_run) = (format!("{ed} - {et}{o}"), format!("{} - {et}{o}", self.edr));
        let mut drill = Vec::new();
        for (part, [x, y], into, across) in rows {
            let rows = PinRows {
                part,
                position: kit.at(&self.point(x), y, &z),
                into,
                depth: &blind,
                across,
                run: if part == return_end {
                    &return_run
                } else {
                    &run
                },
            };
            drill.extend(fittings::pin_rows(kit, top_row, rows));
        }
        drill
    }
}

#[cfg(test)]
mod tests;

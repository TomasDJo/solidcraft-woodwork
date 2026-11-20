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

use crate::fittings::{self, PinRows, PIN_SETBACK, PITCH};
use crate::kit::{mm, Kit};
use crate::Commands;
pub use input::hand;
use input::Input;
use serde_json::{json, Value};

const KICK_RECESS: f64 = 50.0; // plinth boards, back from the fronts

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
    dr: f64, // depth at the return's end: `d` on the L
    g: f64,  // the reveal around the doors, and its parameter
    er: String,
    ea: String,
    eb: String,
    ed: String,
    edr: String,
    eh: String,
    et: String,
    epl: String,
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
        dr,
        shelves,
        pins,
        on_return,
        handles,
        handle_at,
        right,
        diagonal,
        reveal,
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
        dr,
        g: reveal,
        er,
        ea,
        eb,
        ed,
        edr,
        eh,
        et,
        epl,
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
        panel(
            back_end,
            [&format!("{ea} - {et}"), &format!("{eb} - {ed}"), &zero],
            [et, ed, eh],
            None,
        );
        panel(return_end, [&zero, &zero, &zero], [edr, et, eh], None);
        // Full panels against both walls, standing on the floor like the sides.
        panel(
            "Back",
            [et, &format!("{eb} - {et}"), &zero],
            [&format!("{ea} - 2*{et}"), et, eh],
            None,
        );
        panel(
            "Back return",
            [&zero, et, &zero],
            [et, &format!("{eb} - {et}"), eh],
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
                        "edges": [edge], "distance": format!("{eb} - {ed} - 2*{et}"),
                        "body": format!("{name} {part}"), "name": format!("{name} {part} corner"),
                    }),
                ));
                continue;
            }
            panel(
                &format!("{part} notch"),
                [ed, et, z],
                [
                    &format!("{ea} - {et} - {ed}"),
                    &format!("{eb} - {ed} - {et}"),
                    et,
                ],
                Some(&[format!("{name} {part}")]),
            );
        }
        if self.plinth > 0.0 && self.diagonal {
            diagonal::plinth(self);
        } else if self.plinth > 0.0 {
            // Two boards set back from the fronts, meeting in the inner corner.
            let x = format!("{ed} - {} - {et}", mm(KICK_RECESS));
            let y = format!("{eb} - {ed} + {}", mm(KICK_RECESS));
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
        let (near, far) = (
            format!("{et} + {}", mm(PIN_SETBACK)),
            format!("{eb} - {ed} + {}", mm(PIN_SETBACK)),
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
        let (run, return_run) = (format!("{ed} - {et}"), format!("{} - {et}", self.edr));
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

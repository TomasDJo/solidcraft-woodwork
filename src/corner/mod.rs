//! L-shaped corner cabinet: one leg along the back wall, the other (the return) running
//! forward along a side wall.
//!
//! Written for a corner at the back left: the footprint's bounding box is `width` (X) by
//! `return` (Y, the front at 0), and the return is on the left. A right corner is the same
//! mirrored in X. Like the box cabinet, every size is an expression over design parameters.
//!
//! This file is the carcass; `doors.rs` is the bi-fold pair.

mod doors;

use crate::fittings::{self, PinRows, PIN_SETBACK, PITCH};
use crate::kit::{check, mm, num, Kit};
use crate::Commands;
use serde_json::Value;

const KICK_RECESS: f64 = 50.0; // plinth boards, back from the fronts

/// What the steps of a build share: the checked input in mm, and the sizes as expressions.
struct Corner<'a> {
    kit: &'a Kit,
    right: bool,
    handles: &'a str,
    a: f64,
    b: f64,
    d: f64,
    h: f64,
    t: f64,
    plinth: f64,
    ea: String,
    eb: String,
    ed: String,
    eh: String,
    et: String,
    epl: String,
}

pub fn build(p: &Value) -> Result<(Commands, Value), String> {
    let kit = Kit::new(p)?;
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
    // The bi-fold pair hangs on the end of the back leg (`width`) or of the return.
    let on_return = match p.get("doors").and_then(Value::as_str).unwrap_or("none") {
        "none" => None,
        "hinged on width" => Some(false),
        "hinged on return" => Some(true),
        _ => return Err("doors must be none, hinged on width or hinged on return".into()),
    };
    let doors = on_return.is_some();
    let handles = p.get("handles").and_then(Value::as_str).unwrap_or("none");
    check(
        matches!(handles, "none" | "knob" | "pull"),
        "handles must be none, knob or pull",
    )?;
    let right = match p.get("corner").and_then(Value::as_str).unwrap_or("left") {
        "left" => false,
        "right" => true,
        _ => return Err("corner must be left or right".into()),
    };

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

    let ea = kit.parameter("width", a, "along the back wall");
    let eb = kit.parameter("return", b, "along the side wall");
    let ed = kit.parameter("depth", d, "depth of both legs");
    let eh = kit.parameter("height", h, "outside height, plinth included");
    let et = kit.parameter("thickness", t, "panel thickness");
    let epl = match plinth > 0.0 {
        true => kit.parameter("plinth", plinth, "plinth height"),
        false => mm(0.0),
    };
    kit.centre(&ea, &eb);
    let corner = Corner {
        kit: &kit,
        right,
        handles,
        a,
        b,
        d,
        h,
        t,
        plinth,
        ea,
        eb,
        ed,
        eh,
        et,
        epl,
    };
    let drill = corner.carcass(shelves as u32, pins); // holes, run after the panels exist
    if let Some(on_return) = on_return {
        doors::build(&corner, on_return);
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
        panel(
            back_end,
            [&format!("{ea} - {et}"), &format!("{eb} - {ed}"), &zero],
            [et, ed, eh],
            None,
        );
        panel(return_end, [&zero, &zero, &zero], [ed, et, eh], None);
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
        // corner cut out of it. The cut list gives the board before the cut.
        let ech = format!("({eh} - {epl} - 2*{et})"); // clear height
        let egap = format!("(({ech} - {s}*{et}) / {})", s + 1); // between shelves
        let mut boards = vec![
            ("Bottom".to_string(), epl.clone()),
            ("Top".to_string(), format!("{eh} - {et}")),
        ];
        let elast = fittings::top_pin_row(&ech);
        for i in 0..s {
            // Height over the bottom panel: evenly spaced, or on the nearest row of pins.
            let mut over = format!("{egap}*{} + {et}*{i}", i + 1);
            if pins {
                over = fittings::on_pin_row(&over, &elast);
            }
            boards.push((format!("Shelf {}", i + 1), format!("{epl} + {et} + {over}")));
        }
        for (part, z) in &boards {
            panel(
                part,
                [et, et, z],
                [&format!("{ea} - 2*{et}"), &format!("{eb} - 2*{et}"), et],
                None,
            );
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
        if self.plinth > 0.0 {
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
        let run = format!("{ed} - {et}");
        let mut drill = Vec::new();
        for (part, [x, y], into, across) in rows {
            let rows = PinRows {
                part,
                position: kit.at(&self.point(x), y, &z),
                into,
                depth: &blind,
                across,
                run: &run,
            };
            drill.extend(fittings::pin_rows(kit, top_row, rows));
        }
        drill
    }
}

#[cfg(test)]
mod tests;

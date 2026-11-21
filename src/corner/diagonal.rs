//! The diagonal corner cabinet's front: one door at 45 degrees across the open corner, and
//! the plinth board behind it.
//!
//! Both are built square in a component of their own and that component is turned. Its X then
//! runs along the front, from the return's end in a left corner and from the back leg's end
//! in a right one, and its -Y faces the room.

use super::Corner;
use crate::fittings::{self, CUP_CENTRE, CUP_D, CUP_DEPTH, HANDLE_D, HANDLE_EDGE};
use crate::kit::mm;
use serde_json::{json, Value};

impl Corner<'_> {
    /// Where the front starts, as the app gets it and in mm from the cabinet's origin, and
    /// its turn in degrees.
    fn front(&self) -> (Value, [f64; 2], i32) {
        let Corner {
            kit, eb, ed, edr, ..
        } = self;
        let (a, b, zero) = (self.a, self.b, mm(0.0));
        match self.right {
            false => (kit.at(edr, &zero, &zero), [self.dr - a / 2.0, -b / 2.0], 45),
            true => (
                kit.at(&zero, &format!("{eb} - {ed}"), &zero),
                [-a / 2.0, b / 2.0 - self.d],
                -45,
            ),
        }
    }

    /// The length of the front, between the outer corners of the two ends.
    fn front_length(&self) -> String {
        format!("(sqrt(2) * ({} - {}))", self.eb, self.ed)
    }
}

/// One door across the front, hinged at the end of the back leg or (`on_return`) of the
/// return, with the handle at the other end. Overlay in front of the ends; inset in the
/// opening between them, its front face on the front's line.
pub fn door(corner: &Corner, on_return: bool) {
    let Corner {
        kit,
        handles,
        handle_at,
        eh,
        et,
        epl,
        ..
    } = corner;
    let Corner { d, h, plinth, .. } = *corner;
    let (g, gap) = (corner.g, &corner.er);
    // Overlay, each end stops short by the door's own thickness, so its outer corner stays
    // inside the cabinet's footprint and clear of a neighbour's door. Inset, the end panels
    // stand at 45 degrees to the door and reach t/sqrt(2) along it at most (their inner
    // corner, t/sqrt(2) back): the reveal from there. `back` and `front` are the door's
    // faces.
    let (o, inset, [back, front], o_mm) = match corner.inset {
        false => (
            gap.to_string(),
            format!("({et} + 2*{gap})"),
            [format!("-{gap}"), format!("-{gap} - {et}")],
            g,
        ),
        true => (
            format!("({et} + {gap})"),
            format!("({et}/sqrt(2) + {gap})"),
            [et.to_string(), mm(0.0)],
            corner.t + g,
        ),
    };
    let (z0, z1) = (format!("{epl} + {o}"), format!("{eh} - {o}"));
    let door_h = format!("{eh} - {epl} - 2*{o}");
    let len = format!("{} - 2*{inset}", corner.front_length());
    // The front runs towards the back leg's end in a left corner, away from it in a right one.
    let at_end = on_return == corner.right;
    let (x0, x1) = (inset.clone(), format!("{inset} + {len}"));
    let (cup_x, pull_x) = match at_end {
        true => (
            format!("{x1} - {}", mm(CUP_CENTRE)),
            format!("{x0} + {}", mm(HANDLE_EDGE)),
        ),
        false => (
            format!("{x0} + {}", mm(CUP_CENTRE)),
            format!("{x1} - {}", mm(HANDLE_EDGE)),
        ),
    };
    kit.open("Door");
    kit.panel_at("Door", json!([x0, &front, z0]), [&len, et, &door_h], None);
    let hole = |x: &str, z: &str, d: f64, depth: &str, what: String| {
        fittings::hole(
            kit,
            "Door",
            json!([x, back, z]),
            [0, -1, 0],
            d,
            depth,
            &what,
        );
    };
    let cups = fittings::cup_heights(&z0, &z1, h - plinth - 2.0 * o_mm);
    for (j, z) in cups.iter().enumerate() {
        let what = format!("hinge cup {}", j + 1);
        hole(&cup_x, z, CUP_D, &mm(CUP_DEPTH), what);
    }
    let handle_z = fittings::handle_height(
        handles,
        handle_at,
        &z0,
        &z1,
        h - plinth,
        fittings::floor_unit(plinth, d),
    );
    let screws = fittings::screw_heights(handles, &handle_z);
    for (j, z) in screws.iter().enumerate() {
        hole(&pull_x, z, HANDLE_D, et, format!("handle {}", j + 1));
    }
    let (start, _, degrees) = corner.front();
    // The hinge, like the box cabinet's: about the door's front edge at its hinged end. The
    // door is turned, so the line is given twice: along the door, and where that is in the
    // cabinet.
    let x = match at_end {
        true => format!("{inset} + {len}"),
        false => inset.clone(),
    };
    let y = front;
    let (sin, cos) = (degrees as f64).to_radians().sin_cos();
    let from = |i: usize| start[i].as_str().unwrap_or_default().to_string();
    let seat = [
        format!("{} + ({x}) * ({cos}) - ({y}) * ({sin})", from(0)),
        format!("{} + ({x}) * ({sin}) + ({y}) * ({cos})", from(1)),
    ];
    let turn = if at_end { 1 } else { -1 };
    kit.hinge_line("Door", "hinge", [&x, &y], turn);
    kit.turn("Door", start, degrees);
    let own = Some(degrees);
    kit.hinge(
        "Door",
        None,
        "hinge",
        [&seat[0], &seat[1]],
        own,
        turn,
        110.0,
    );
}

/// One board along the front, set back from it like the L's.
pub fn plinth(corner: &Corner) {
    let Corner {
        kit,
        et,
        epl,
        ekick,
        ..
    } = corner;
    kit.open("Plinth");
    kit.panel_at(
        "Plinth",
        json!([mm(0.0), ekick, mm(0.0)]),
        [&corner.front_length(), et, epl],
        None,
    );
    let (start, _, degrees) = corner.front();
    kit.turn("Plinth", start, degrees);
    kit.fix("Plinth");
}

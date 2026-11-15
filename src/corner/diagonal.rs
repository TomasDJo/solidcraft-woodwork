//! The diagonal corner cabinet's front: one door at 45 degrees across the open corner, and
//! the plinth board behind it.
//!
//! Both are built square in a component of their own and that component is turned. Its X then
//! runs along the front, from the return's end in a left corner and from the back leg's end
//! in a right one, and its -Y faces the room.

use super::{Corner, KICK_RECESS};
use crate::fittings::{self, CUP_CENTRE, CUP_D, CUP_DEPTH, DOOR_GAP, HANDLE_D, HANDLE_EDGE};
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

/// One overlay door across the front, hinged at the end of the back leg or (`on_return`) of
/// the return, with the handle at the other end.
pub fn door(corner: &Corner, on_return: bool) {
    let Corner {
        kit,
        handles,
        eh,
        et,
        epl,
        ..
    } = corner;
    let Corner {
        b, d, h, t, plinth, ..
    } = *corner;
    let (g, gap) = (DOOR_GAP, mm(DOOR_GAP));
    let (z0, z1) = (format!("{epl} + {gap}"), format!("{eh} - {gap}"));
    let door_h = format!("{eh} - {epl} - {}", mm(2.0 * g));
    // Each end stops short by the door's own thickness, so its outer corner stays inside the
    // cabinet's footprint and clear of a neighbour's door.
    let inset = format!("({et} + {})", mm(2.0 * g));
    let inset_mm = t + 2.0 * g;
    let len = format!("{} - 2*{inset}", corner.front_length());
    let len_mm = 2f64.sqrt() * (b - d) - 2.0 * inset_mm;
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
    let back = format!("-{gap}"); // the door's back face
    kit.open("Door");
    kit.panel_at(
        "Door",
        json!([x0, format!("{back} - {et}"), z0]),
        [&len, et, &door_h],
        None,
    );
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
    let cups = fittings::cup_heights(&z0, &z1, h - plinth - 2.0 * g);
    for (j, z) in cups.iter().enumerate() {
        let what = format!("hinge cup {}", j + 1);
        hole(&cup_x, z, CUP_D, &mm(CUP_DEPTH), what);
    }
    let handle_z = fittings::handle_height(
        handles,
        &z0,
        &z1,
        h - plinth,
        fittings::floor_unit(plinth, d),
    );
    let screws = fittings::screw_heights(handles, &handle_z);
    for (j, z) in screws.iter().enumerate() {
        hole(&pull_x, z, HANDLE_D, et, format!("handle {}", j + 1));
    }
    let (start, [ox, oy], degrees) = corner.front();
    kit.turn("Door", start, degrees);
    // The hinge, like the box cabinet's: about the door's front edge at its hinged end.
    let (x, y) = (inset_mm + if at_end { len_mm } else { 0.0 }, -g - t);
    let (sin, cos) = (degrees as f64).to_radians().sin_cos();
    let hinge = [ox + x * cos - y * sin, oy + x * sin + y * cos, plinth + g];
    let turn = if at_end { 1 } else { -1 };
    kit.close(
        "Door",
        None,
        "revolute",
        "hinge",
        hinge,
        [0, 0, turn],
        110.0,
    );
}

/// One board along the front, set back from it like the L's.
pub fn plinth(corner: &Corner) {
    let Corner { kit, et, epl, .. } = corner;
    kit.open("Plinth");
    kit.panel_at(
        "Plinth",
        json!([mm(0.0), mm(KICK_RECESS), mm(0.0)]),
        [&corner.front_length(), et, epl],
        None,
    );
    let (start, _, degrees) = corner.front();
    kit.turn("Plinth", start, degrees);
    kit.fix("Plinth");
}

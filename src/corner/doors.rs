//! The corner cabinet's doors: a bi-fold pair, the second hung on the first.

use super::Corner;
use crate::fittings::{self, CUP_CENTRE, CUP_D, CUP_DEPTH, DOOR_GAP, HANDLE_D, HANDLE_EDGE};
use crate::kit::mm;

/// A bi-fold pair of overlay doors, one on each front. The first hinges on the end of its leg
/// and runs into the inner corner, closing it; the second stops against the first's face,
/// hangs on it there and folds out flat with it. `on_return`: the pair hangs on the return.
pub fn build(corner: &Corner, on_return: bool) {
    let Corner {
        kit,
        handles,
        ea,
        eb,
        ed,
        eh,
        et,
        epl,
        ..
    } = corner;
    let Corner {
        a,
        b,
        d,
        h,
        t,
        plinth,
        ..
    } = *corner;
    let flip = corner.flip();
    let gap = mm(DOOR_GAP);
    // Each door's end at the inner corner: the first's, and the second's a panel and a
    // gap short of it.
    let (long_x, short_x) = (
        format!("{ed} + {gap}"),
        format!("{ed} + {et} + {}", mm(2.0 * DOOR_GAP)),
    );
    let (long_y, short_y) = (
        format!("{eb} - {ed} - {gap}"),
        format!("{eb} - {ed} - {et} - {}", mm(2.0 * DOOR_GAP)),
    );
    let (z0, z1) = (format!("{epl} + {gap}"), format!("{eh} - {gap}"));
    let door_h = format!("{eh} - {epl} - {}", mm(2.0 * DOOR_GAP));
    let cups = fittings::cup_heights(&z0, &z1, h - plinth - 2.0 * DOOR_GAP);
    // The first door carries the handle: upright on its opening edge, at the height the
    // box cabinet would put it.
    let handle_z = fittings::handle_height(handles, &z0, &z1, h - plinth, plinth > 0.0);
    let screws = fittings::screw_heights(handles, &handle_z);
    // Holes from a door's back face: hinge cups at `cup`, and on the first door the handle
    // screws at `pull`. Both are x and y; `into` is the way into the door.
    let holes = |part: &str, cup: [&str; 2], pull: [&str; 2], first: bool, into: [i32; 3]| {
        let hole = |at: [&str; 2], z: &str, d: f64, depth: &str, what: String| {
            let position = kit.at(&corner.point(at[0]), at[1], z);
            fittings::hole(kit, part, position, into, d, depth, &what);
        };
        for (j, z) in cups.iter().enumerate() {
            hole(
                cup,
                z,
                CUP_D,
                &mm(CUP_DEPTH),
                format!("hinge cup {}", j + 1),
            );
        }
        for (j, z) in screws.iter().enumerate().filter(|_| first) {
            hole(pull, z, HANDLE_D, et, format!("handle {}", j + 1));
        }
    };
    let (g, floor) = (DOOR_GAP, plinth + DOOR_GAP);
    let (cup_c, edge) = (mm(CUP_CENTRE), mm(HANDLE_EDGE));
    // On the back leg, facing the front.
    let face_y = &long_y; // the back leg door's back face
    let door_x = if on_return { &short_x } else { &long_x };
    let door = || {
        kit.open("Door");
        corner.panel(
            "Door",
            [door_x, &format!("{face_y} - {et}"), &z0],
            [&format!("{ea} - {gap} - ({door_x})"), et, &door_h],
            None,
        );
        let cup_x = match on_return {
            true => format!("{short_x} + {cup_c}"),
            false => format!("{ea} - {gap} - {cup_c}"),
        };
        let pull_x = format!("{short_x} + {edge}");
        holes(
            "Door",
            [&cup_x, face_y],
            [&pull_x, face_y],
            !on_return,
            [0, -1, 0],
        );
    };
    // On the return, facing across.
    let face_x = &long_x; // its back face
    let return_y = if on_return { &long_y } else { &short_y };
    let door_return = || {
        kit.open("Door return");
        corner.panel(
            "Door return",
            [face_x, &gap, &z0],
            [et, &format!("{return_y} - {gap}"), &door_h],
            None,
        );
        let cup_y = match on_return {
            true => format!("{gap} + {cup_c}"),
            false => format!("{short_y} - {cup_c}"),
        };
        let pull_y = format!("{short_y} - {edge}");
        holes(
            "Door return",
            [face_x, &cup_y],
            [face_x, &pull_y],
            on_return,
            [flip, 0, 0],
        );
    };
    // A hinge is about the door's front edge at the end of its leg, like the box cabinet's,
    // so the door clears the next cabinet. The fold is where the two back faces meet, as
    // on a bi-fold hinge: 90 degrees brings the second door in line with the first.
    let (front_y, meet_x) = (b / 2.0 - d - g - t, (d + g + t - a / 2.0) * flip as f64);
    let fold = [meet_x - t * flip as f64, front_y + t, floor];
    let close = |part: &str, on: Option<&str>, joint: &str, at: [f64; 3], turn: i32| {
        let limit = if on.is_some() { 90.0 } else { 110.0 };
        kit.close(part, on, "revolute", joint, at, [0, 0, turn], limit)
    };
    if on_return {
        door_return();
        let hinge = [meet_x, g - b / 2.0, floor];
        close("Door return", None, "hinge", hinge, -flip);
        door();
        close("Door", Some("Door return"), "fold", fold, flip);
    } else {
        door();
        let hinge = [(a / 2.0 - g) * flip as f64, front_y, floor];
        close("Door", None, "hinge", hinge, flip);
        door_return();
        close("Door return", Some("Door"), "fold", fold, -flip);
    }
}

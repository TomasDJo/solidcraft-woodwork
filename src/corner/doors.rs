//! The corner cabinet's doors: a bi-fold pair, the second hung on the first.

use super::Corner;
use crate::fittings::{self, CUP_CENTRE, CUP_D, CUP_DEPTH, DOOR_GAP, HANDLE_D, HANDLE_EDGE};
use crate::kit::mm;

/// A bi-fold pair of overlay doors, one on each front. The first hinges on the end of its leg
/// and runs into the inner corner, closing it; the second stops against the first's face,
/// hangs on it there and folds out onto its face. `on_return`: the pair hangs on the return.
pub fn build(corner: &Corner, on_return: bool) {
    let Corner {
        kit,
        handles,
        handle_at,
        ea,
        eb,
        ed,
        eh,
        et,
        epl,
        ..
    } = corner;
    let Corner { d, h, plinth, .. } = *corner;
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
    // The second door, the one that folds, carries the handle: upright at its free end, away
    // from the fold, at the height the box cabinet would put it.
    let handle_z = fittings::handle_height(
        handles,
        handle_at,
        &z0,
        &z1,
        h - plinth,
        fittings::floor_unit(plinth, d),
    );
    let screws = fittings::screw_heights(handles, &handle_z);
    // Holes from a door's back face: hinge cups at `cup`, and on the second door the handle
    // screws at `pull`. Both are x and y; `into` is the way into the door.
    let holes = |part: &str, cup: [&str; 2], pull: [&str; 2], second: bool, into: [i32; 3]| {
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
        for (j, z) in screws.iter().enumerate().filter(|_| second) {
            hole(pull, z, HANDLE_D, et, format!("handle {}", j + 1));
        }
    };
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
        let pull_x = format!("{ea} - {gap} - {edge}");
        holes(
            "Door",
            [&cup_x, face_y],
            [&pull_x, face_y],
            on_return,
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
        let pull_y = format!("{gap} + {edge}");
        holes(
            "Door return",
            [face_x, &cup_y],
            [face_x, &pull_y],
            !on_return,
            [flip, 0, 0],
        );
    };
    // A hinge is about the door's front edge at the end of its leg, like the box cabinet's,
    // so the door clears the next cabinet. The fold is where the two front faces meet: 90
    // degrees lays the second door on the face of the first, out of the cabinet, so it
    // opens on its own too.
    let front_y = format!("{eb} - {ed} - {gap} - {et}");
    let meet_x = format!("{ed} + {gap} + {et}");
    let close = |part: &str, on: Option<&str>, joint: &str, [x, y]: [&str; 2], turn: i32| {
        let limit = if on.is_some() { 90.0 } else { 110.0 };
        let line = kit.at(&corner.point(x), y, &z0);
        let line = [0, 1].map(|i| line[i].as_str().unwrap_or_default());
        kit.hinge(part, on, joint, line, None, turn, limit)
    };
    let fold = [meet_x.as_str(), front_y.as_str()];
    if on_return {
        door_return();
        close("Door return", None, "hinge", [&meet_x, &gap], -flip);
        door();
        close("Door", Some("Door return"), "fold", fold, -flip);
    } else {
        door();
        let end = format!("{ea} - {gap}");
        close("Door", None, "hinge", [&end, &front_y], flip);
        door_return();
        close("Door return", Some("Door"), "fold", fold, flip);
    }
}

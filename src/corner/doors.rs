//! The corner cabinet's doors: a bi-fold pair, the second hung on the first.

use super::Corner;
use crate::fittings::{self, HANDLE_D};
use crate::hardware;
use crate::kit::mm;

/// A bi-fold pair of doors, one on each front. The first hinges on the end of its leg
/// and runs into the inner corner, closing it; the second stops against the first's face,
/// hangs on it there and folds out onto its face. `on_return`: the pair hangs on the return.
/// Inset, the pair is the same a panel back, inside the opening: front faces flush with the
/// carcass, the reveal from the end panels, the bottom and the top.
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
    let gap = &corner.er;
    // The doors' back faces, the return's (x) and the back leg's (y): a reveal in front of
    // the carcass, or a panel inside it; their front faces; and how far the doors' outer
    // edges are inside the carcass's outside.
    let ([back_x, back_y], [meet_x, front_y], o, o_mm) = match corner.inset {
        false => (
            [format!("{ed} + {gap}"), format!("{eb} - {ed} - {gap}")],
            [
                format!("{ed} + {gap} + {et}"),
                format!("{eb} - {ed} - {gap} - {et}"),
            ],
            gap.to_string(),
            corner.g,
        ),
        true => (
            [format!("{ed} - {et}"), format!("{eb} - {ed} + {et}")],
            [ed.to_string(), format!("{eb} - {ed}")],
            format!("({et} + {gap})"),
            corner.t + corner.g,
        ),
    };
    // Each door's end at the inner corner: the first's, and the second's a panel and a
    // gap short of it.
    let (long_x, short_x) = (back_x.clone(), format!("{back_x} + {et} + {gap}"));
    let (long_y, short_y) = (back_y.clone(), format!("{back_y} - {et} - {gap}"));
    let (z0, z1) = (format!("{epl} + {o}"), format!("{eh} - {o}"));
    let door_h = format!("{eh} - {epl} - 2*{o}");
    // On the pins' grid over the bottom panel, as on the box cabinet.
    let base = format!("{epl} + {et}");
    let cups = hardware::cups(&z0, &z1, h - plinth - 2.0 * o_mm, &base);
    // The second door, the one that folds, carries the handle: upright at its free end, away
    // from the fold, at the height the box cabinet would put it.
    let floor = fittings::floor_unit(plinth, d);
    let handle_z = fittings::handle_height(handles, handle_at, &z0, &z1, h - plinth, floor);
    // Grooves in both doors' faces; a grip edge or groove instead of a handle along the second
    // door's top or bottom.
    let profile = |part: &str, lo: [&str; 3], size: [&str; 3], along: usize, back: i32, second| {
        let at = |x: &str, y: &str, z: &str| kit.at(x, y, z);
        if let Some((g, sizes)) = &corner.grooves {
            fittings::front_grooves(kit, part, g, sizes, at, lo, size, along, back);
        }
        if second {
            let top = fittings::grip_on_top(handle_at, floor);
            fittings::grip(kit, part, handles, at, lo, size, along, back, top);
        }
    };
    // The second door's height: with a grip edge the grip gap short of its grip edge (#230).
    let height = |second: bool| match second {
        true => fittings::gripped(
            corner.egap.as_deref(),
            &z0,
            &door_h,
            fittings::grip_on_top(handle_at, floor),
        ),
        false => (z0.clone(), door_h.clone()),
    };
    let screws = fittings::screw_heights(handles, &handle_z);
    // Holes from a door's back face: hinge cups at `cup`, and on the second door the handle
    // screws at `pull`. Both are x and y; `into` is the way into the door, and from the cups
    // towards its hinge edge.
    // the second door's cups over its own height, shorter with a grip gap
    let (gz, gh) = height(true);
    let second_cups = hardware::cups(
        &gz,
        &format!("{gz} + {gh}"),
        h - plinth - 2.0 * o_mm - corner.gap,
        &base,
    );
    let holes = |part: &str, cup: [&str; 2], pull: [&str; 2], second: bool, into: [[i32; 3]; 2]| {
        let cups = if second { &second_cups } else { &cups };
        let hole = |at: [&str; 2], z: &str, d: f64, depth: &str, what: String| {
            let position = kit.at(&corner.point(at[0]), at[1], z);
            fittings::hole(kit, part, position, into[0], &mm(d), depth, &what);
        };
        let cup_x = corner.point(cup[0]);
        // The second door folds on the first: its cups get no model (`cup_holes`).
        let crank = match (second, corner.inset) {
            (true, _) => None,
            (false, true) => Some(18.0),
            (false, false) => Some(0.0),
        };
        hardware::cup_holes(kit, part, |z| kit.at(&cup_x, cup[1], z), into, crank, cups);
        for (j, z) in screws.iter().enumerate().filter(|_| second) {
            hole(pull, z, HANDLE_D, et, format!("handle {}", j + 1));
        }
    };
    let (cup_c, edge) = (
        "HW_cup_inset",
        fittings::handle_edge(corner.eframe.as_ref()),
    );
    // On the back leg, facing the front.
    let face_y = &long_y; // the back leg door's back face
    let door_x = if on_return { &short_x } else { &long_x };
    let door = || {
        kit.open("Door");
        let (y, w) = (
            format!("{face_y} - {et}"),
            format!("{ea} - {o} - ({door_x})"),
        );
        let x = corner.from(door_x, &w);
        // a shaker door's own name is the stile with its cups: at the fold, or the leg's end
        let at = |x: &str, y: &str, z: &str| kit.at(x, y, z);
        let (z, door_h) = height(on_return);
        let (lo, size) = ([x.as_str(), &y, &z], [w.as_str(), et, &door_h]);
        let main_low = on_return != corner.right;
        fittings::front(
            kit,
            "Door",
            corner.eframe.as_ref(),
            at,
            lo,
            size,
            0,
            main_low,
            corner.glass,
        );
        profile("Door", lo, size, 0, 1, on_return);
        let cup_x = match on_return {
            true => format!("{short_x} + {cup_c}"),
            false => format!("{ea} - {o} - {cup_c}"),
        };
        let pull_x = format!("{ea} - {o} - {edge}");
        holes(
            "Door",
            [&cup_x, face_y],
            [&pull_x, face_y],
            on_return,
            // towards the hinge edge: the end of the leg, or the fold
            [[0, -1, 0], [if on_return { -flip } else { flip }, 0, 0]],
        );
    };
    // On the return, facing across.
    let face_x = &long_x; // its back face
    let return_y = if on_return { &long_y } else { &short_y };
    let door_return = || {
        kit.open("Door return");
        let long = format!("{return_y} - {o}");
        let x = corner.from(face_x, et);
        let (z, door_h) = height(!on_return);
        let (lo, size) = ([x.as_str(), &o, &z], [et.as_str(), &long, &door_h]);
        let at = |x: &str, y: &str, z: &str| kit.at(x, y, z);
        let frame = corner.eframe.as_ref();
        fittings::front(
            kit,
            "Door return",
            frame,
            at,
            lo,
            size,
            1,
            on_return,
            corner.glass,
        );
        profile("Door return", lo, size, 1, -flip, !on_return);
        let cup_y = match on_return {
            true => format!("{o} + {cup_c}"),
            false => format!("{short_y} - {cup_c}"),
        };
        let pull_y = format!("{o} + {edge}");
        holes(
            "Door return",
            [face_x, &cup_y],
            [face_x, &pull_y],
            !on_return,
            [[flip, 0, 0], [0, if on_return { -1 } else { 1 }, 0]],
        );
    };
    // A hinge is about the door's front edge at the end of its leg, like the box cabinet's,
    // so the door clears the next cabinet. The fold is where the two front faces meet: 90
    // degrees lays the second door on the face of the first, out of the cabinet, so it
    // opens on its own too.
    let close = |part: &str, on: Option<&str>, joint: &str, [x, y]: [&str; 2], turn: i32| {
        // The fold lays the second door flat on the first, whatever the hinge.
        let limit = if on.is_some() { 90.0 } else { kit.hinge_angle };
        let line = kit.at(&corner.point(x), y, &z0);
        let line = [0, 1].map(|i| line[i].as_str().unwrap_or_default());
        kit.hinge(part, on, joint, line, None, turn, limit)
    };
    // The plates' screw holes in the end the pair hangs on, `HW_setback` behind the front of
    // the opening (the pin rows' line), as on the box cabinet: cut after the hinge, with the
    // cabinet active again.
    let (back_end, return_end) = corner.ends();
    let (open_x, open_y) = match corner.inset {
        false => (ed.to_string(), format!("{eb} - {ed}")),
        true => (format!("{ed} - {et}"), format!("{eb} - {ed} + {et}")),
    };
    let plates = |on_return: bool| {
        let (board, x, y, into) = match on_return {
            false => (
                back_end,
                format!("{ea} - {et}"),
                format!("{open_y} + HW_setback"),
                [flip, 0, 0],
            ),
            true => (
                return_end,
                format!("{open_x} - HW_setback"),
                et.to_string(),
                [0, -1, 0],
            ),
        };
        let x = corner.point(&x);
        hardware::plate_holes(kit, board, |z| kit.at(&x, &y, z), into, &cups, et);
    };
    let fold = [meet_x.as_str(), front_y.as_str()];
    if on_return {
        door_return();
        close("Door return", None, "hinge", [&meet_x, &o], -flip);
        plates(true);
        door();
        close("Door", Some("Door return"), "fold", fold, -flip);
    } else {
        door();
        let end = format!("{ea} - {o}");
        close("Door", None, "hinge", [&end, &front_y], flip);
        plates(false);
        door_return();
        close("Door return", Some("Door"), "fold", fold, flip);
    }
}

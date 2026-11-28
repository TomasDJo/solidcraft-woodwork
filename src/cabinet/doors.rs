//! Doors on cup hinges, overlay or inset, each a component of its own on a revolute joint.

use super::input::Input;
use super::Cabinet;
use crate::fittings;
use crate::hardware;

/// Equal doors across the front. They hinge left, right, left..., or right, left, right...
/// when the first is asked to hinge on its right; with three or more the end doors hinge on
/// the sides whatever is asked (`hinged_left`), so as few as can hinge in the middle, on a
/// divider. Two doors are a pair that opens from the middle whatever is asked: hinged the
/// other way both would hang in the middle. Overlay, one door per compartment puts every gap
/// on a divider (within half a panel), where a door covers half of it: one hinged there gets
/// half overlay hinges; inset, each door fills its own compartment.
pub fn build(cabinet: &Cabinet) {
    let Cabinet {
        kit,
        ew,
        et,
        er,
        ein,
        ecell,
        efront,
        eback,
        edoor: [z0, z1, door_h],
        ..
    } = cabinet;
    let Input {
        d,
        plinth,
        doors,
        dividers,
        door_handles: handles,
        handle_at,
        hinge_right,
        inset,
        double,
        door_h: door_h_mm,
        ..
    } = cabinet.input;
    let count = doors as usize;
    let first = usize::from(hinge_right && count != 2); // 1: the first door hinges on its right
    let per_cell = inset && dividers > 0.0 && doors == dividers + 1.0;
    let door_w = match per_cell {
        true => format!("({ecell} - 2*{er})"),
        false => format!(
            "(({ew} - 2*{ein} - 2*{er} - {}*{er}) / {count})",
            count.max(1) - 1
        ),
    };
    // From the plinth (or the bottom panel) up to the top (or under the top panel), or to
    // the drawers.
    // On the pin rows' grid, so that each plate's two screw holes go into the system holes.
    let (base, _) = cabinet.shelf_zone();
    let cups = hardware::cups(z0, z1, door_h_mm, &base);
    let floor = fittings::floor_unit(plinth, d);
    let handle_z = fittings::handle_height(handles, handle_at, z0, z1, door_h_mm, floor);
    let grip_top = fittings::grip_on_top(handle_at, floor);
    for i in 0..count {
        let part = match count {
            1 => "Door".to_string(),
            2 => ["Door L", "Door R"][i].to_string(),
            _ => format!("Door {}", i + 1),
        };
        let x0 = match per_cell {
            true => format!("{} + {er}", cabinet.cell_x(i as u32)),
            false => format!("{ein} + {er} + ({door_w} + {er}) * {i}"),
        };
        kit.open(&part);
        // a shaker door's own name is its hinge stile's, which takes the cups
        let left = hinged_left(i, count, first, inset);
        // a grip edge's door stops the grip gap short of its grip edge (#230)
        let (gz, gh) = cabinet.gripped_door(z0, door_h, grip_top);
        cabinet.front(&part, [&x0, efront, &gz], [&door_w, et, &gh], left, true);
        // The hinge cups, from the door's back face, `HW_cup_inset` in from its hinge edge.
        let cup_x = match left {
            true => format!("{x0} + HW_cup_inset"),
            false => format!("{x0} + {door_w} - HW_cup_inset"),
        };
        let at = |z: &str| kit.at(&cup_x, eback, z);
        let toward = [if left { -1 } else { 1 }, 0, 0];
        // The hinge's arm: straight to a side or a double divider's board, cranked 9.5 mm to a
        // divider (half overlay), or 18 mm for an inset door.
        let on_side = matches!(
            (left, i == 0, i + 1 == count),
            (true, true, _) | (false, _, true)
        );
        let crank = match (inset, on_side) {
            (true, _) => 18.0,
            (false, false) if !double => hardware::HALF_OVERLAY,
            (false, _) => 0.0,
        };
        hardware::cup_holes(kit, &part, at, [[0, -1, 0], toward], Some(crank), &cups);
        // The handle is on the side away from the hinges.
        let edge = fittings::handle_edge(cabinet.eframe.as_ref());
        let handle_x = match left {
            true => format!("{x0} + {door_w} - {edge}"),
            false => format!("{x0} + {edge}"),
        };
        cabinet.handle(&part, &handle_x, &handle_z, 2);
        cabinet.grooves(&part, [&x0, efront, &gz], [&door_w, et, &gh]);
        cabinet.grip(&part, [&x0, efront, &gz], [&door_w, et, &gh], grip_top);
        // The hinge: turning outwards up to `hinge_angle` (110 degrees by default) about the
        // front edge on the door's hinge side. A cup hinge carries the door out and round so that its edge clears the door of
        // the next cabinet; about the back edge it would swing a panel thickness into it. An
        // inset door's back edge then moves away from the side as it opens.
        let (x, turn) = match left {
            true => (x0.clone(), -1),
            false => (format!("{x0} + {door_w}"), 1),
        };
        let line = kit.at(&x, efront, z0);
        let line = [0, 1].map(|i| line[i].as_str().unwrap_or_default());
        kit.hinge(&part, None, "hinge", line, None, turn, kit.hinge_angle);
        // The plates' screw holes in the side the door hangs on: `HW_pitch` apart about each
        // cup's height, `HW_setback` behind the front of the opening (the shelf pins' line).
        // ponytail: a hinge edge on a single divider (three doors or more) gets no plate holes:
        // the next door's hinges could want the same place. A double divider's board is the
        // door's own (#176): door i hangs on divider i's right board or divider i + 1's left.
        let side = match (left, i == 0, i + 1 == count) {
            (true, true, _) => Some(("Side L".to_string(), et.clone(), -1)),
            (false, _, true) => Some(("Side R".to_string(), format!("{ew} - {et}"), 1)),
            (true, ..) if double => Some((format!("Divider {i} R"), cabinet.cell_x(i as u32), -1)),
            (false, ..) if double => {
                let x = format!("{} - 2*{et}", cabinet.cell_x(i as u32 + 1));
                Some((format!("Divider {} L", i + 1), x, 1))
            }
            _ => None,
        };
        if let Some((board, x, into)) = side {
            let y = format!("{ein} + HW_setback");
            let at = |z: &str| kit.at(&x, &y, z);
            hardware::plate_holes(kit, &board, at, [into, 0, 0], &cups, et);
        }
    }
}

/// Whether door `i` of `count` hinges on its left: alternating from the first, which hinges
/// on its right when `first` is 1. With three doors or more the first hinges on Side L and
/// the last on Side R whatever is asked, so only the doors between hinge on a divider, and
/// `first` is the side the second door hinges on, the rest alternating from it. Inset doors
/// each fill their own compartment, so they alternate from the first as asked.
pub fn hinged_left(i: usize, count: usize, first: usize, inset: bool) -> bool {
    let ends = count >= 3 && !inset;
    match (ends, i) {
        (true, 0) => true,
        (true, i) if i + 1 == count => false,
        _ => (i + first + usize::from(ends)).is_multiple_of(2),
    }
}

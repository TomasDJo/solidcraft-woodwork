//! Overlay doors on cup hinges, each a component of its own on a revolute joint.

use super::input::Input;
use super::Cabinet;
use crate::fittings::{self, CUP_CENTRE, CUP_D, CUP_DEPTH, DOOR_GAP, HANDLE_EDGE};
use crate::kit::mm;

/// Equal overlay doors across the front. They hinge left, right, left... so pairs open from
/// the middle. One door per compartment puts every gap on a divider (within half a panel).
pub fn build(cabinet: &Cabinet) {
    let Cabinet {
        kit,
        ew,
        eh,
        et,
        epl,
        ..
    } = cabinet;
    let Input {
        w,
        h,
        d,
        t,
        plinth,
        doors,
        handles,
        ..
    } = cabinet.input;
    let count = doors as usize;
    let gap_mm = mm(DOOR_GAP);
    let door_w = format!(
        "(({ew} - {} - {}*{gap_mm}) / {count})",
        mm(2.0 * DOOR_GAP),
        count.max(1) - 1
    );
    let door_h = format!("{eh} - {epl} - {}", mm(2.0 * DOOR_GAP));
    let (z0, z1) = (format!("{epl} + {gap_mm}"), format!("{eh} - {gap_mm}"));
    let cups = fittings::cup_heights(&z0, &z1, h - plinth - 2.0 * DOOR_GAP);
    let handle_z = fittings::handle_height(handles, &z0, &z1, h - plinth, plinth > 0.0);
    for i in 0..count {
        let part = match count {
            1 => "Door".to_string(),
            2 => ["Door L", "Door R"][i].to_string(),
            _ => format!("Door {}", i + 1),
        };
        let x0 = format!("{gap_mm} + ({door_w} + {gap_mm}) * {i}");
        let y = format!("-{gap_mm} - {et}");
        kit.open(&part);
        kit.panel(&part, [&x0, &y, &z0], [&door_w, et, &door_h], None);
        let cup_x = match i % 2 {
            0 => format!("{x0} + {}", mm(CUP_CENTRE)),
            _ => format!("{x0} + {door_w} - {}", mm(CUP_CENTRE)),
        };
        for (j, z) in cups.iter().enumerate() {
            let position = kit.at(&cup_x, &format!("-{gap_mm}"), z);
            let what = format!("hinge cup {}", j + 1);
            fittings::hole(
                kit,
                &part,
                position,
                [0, -1, 0],
                CUP_D,
                &mm(CUP_DEPTH),
                &what,
            );
        }
        // The handle is on the side away from the hinges.
        let handle_x = match i % 2 {
            0 => format!("{x0} + {door_w} - {}", mm(HANDLE_EDGE)),
            _ => format!("{x0} + {}", mm(HANDLE_EDGE)),
        };
        cabinet.handle(&part, &handle_x, &handle_z, 2);
        // The hinge: turning outwards up to 110 degrees about the front edge on the door's hinge
        // side. A cup hinge carries the door out and round so that its edge clears the door of
        // the next cabinet; about the back edge it would swing a panel thickness into it.
        let width = (w - 2.0 * DOOR_GAP - (doors - 1.0) * DOOR_GAP) / doors;
        let left = -w / 2.0 + DOOR_GAP + (width + DOOR_GAP) * i as f64;
        let (x, axis) = if i % 2 == 0 {
            (left, -1)
        } else {
            (left + width, 1)
        };
        let point = [x, -d / 2.0 - DOOR_GAP - t, plinth + DOOR_GAP];
        kit.close(&part, None, "revolute", "hinge", point, [0, 0, axis], 110.0);
    }
}

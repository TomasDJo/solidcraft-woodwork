//! Doors on cup hinges, overlay or inset, each a component of its own on a revolute joint.

use super::input::Input;
use super::Cabinet;
use crate::fittings::{self, CUP_CENTRE, CUP_D, CUP_DEPTH, HANDLE_EDGE};
use crate::kit::mm;

/// Equal doors across the front. They hinge left, right, left..., or right, left, right...
/// when the first is asked to hinge on its right. Two doors are a pair that opens from the
/// middle whatever is asked: hinged the other way both would hang in the middle. Overlay, one
/// door per compartment puts every gap on a divider (within half a panel); inset, each door
/// fills its own compartment.
pub fn build(cabinet: &Cabinet) {
    let Cabinet {
        kit,
        ew,
        et,
        epl,
        er,
        ein,
        ecell,
        efront,
        eback,
        ez1,
        ..
    } = cabinet;
    let Input {
        door_top,
        d,
        plinth,
        doors,
        dividers,
        handles,
        handle_at,
        hinge_right,
        inset,
        reveal,
        t,
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
    let o = if inset { t } else { 0.0 };
    // From the plinth (or the bottom panel) up to the top, or to under the drawers.
    let door_h = format!("{ez1} - {epl} - {ein} - {er}");
    let (z0, z1) = (format!("{epl} + {ein} + {er}"), ez1.clone());
    let door_h_mm = door_top - plinth - o - reveal;
    let cups = fittings::cup_heights(&z0, &z1, door_h_mm);
    let handle_z = fittings::handle_height(
        handles,
        handle_at,
        &z0,
        &z1,
        door_h_mm,
        fittings::floor_unit(plinth, d),
    );
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
        kit.panel(&part, [&x0, efront, &z0], [&door_w, et, &door_h], None);
        let cup_x = match (i + first) % 2 {
            0 => format!("{x0} + {}", mm(CUP_CENTRE)),
            _ => format!("{x0} + {door_w} - {}", mm(CUP_CENTRE)),
        };
        for (j, z) in cups.iter().enumerate() {
            let position = kit.at(&cup_x, eback, z);
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
        let handle_x = match (i + first) % 2 {
            0 => format!("{x0} + {door_w} - {}", mm(HANDLE_EDGE)),
            _ => format!("{x0} + {}", mm(HANDLE_EDGE)),
        };
        cabinet.handle(&part, &handle_x, &handle_z, 2);
        // The hinge: turning outwards up to 110 degrees about the front edge on the door's hinge
        // side. A cup hinge carries the door out and round so that its edge clears the door of
        // the next cabinet; about the back edge it would swing a panel thickness into it. An
        // inset door's back edge then moves away from the side as it opens.
        let (x, turn) = match (i + first) % 2 {
            0 => (x0.clone(), -1),
            _ => (format!("{x0} + {door_w}"), 1),
        };
        let line = kit.at(&x, efront, &z0);
        let line = [0, 1].map(|i| line[i].as_str().unwrap_or_default());
        kit.hinge(&part, None, "hinge", line, None, turn, 110.0);
    }
}

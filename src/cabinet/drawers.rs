//! Drawers: an overlay front with a box behind it, a component of its own on a slider joint.

use super::input::Input;
use super::Cabinet;
use crate::fittings::DOOR_GAP;
use crate::kit::mm;

pub const SLIDE: f64 = 12.7; // ball-bearing drawer slide, each side
const BOX_T: f64 = 12.0; // drawer box sides, front and back
const BOX_BOTTOM: f64 = 6.0;
// The box stays a panel thickness and this much inside its front's lower and upper edge: the
// lowest box then clears the cabinet's bottom, and the highest its top, by this and the gap.
pub const BOX_CLEAR: f64 = 8.0;
const BOX_BEHIND: f64 = 10.0; // room behind the box
const HANDLE_EDGE: f64 = 40.0; // a handle at the top or bottom: from the front's edge

/// A stack of drawers: an overlay front, and behind it a box that runs on slides. They are
/// alike, or the top one is `drawer_height` and the rest share what is left, or over doors
/// all are `drawer_height`.
pub fn build(cabinet: &Cabinet) {
    let Cabinet {
        kit,
        ew,
        eh,
        et,
        epl,
        eiw,
        eshelf_d,
        ez1,
        edh,
        ..
    } = cabinet;
    let Input {
        d,
        drawers,
        doors,
        shelf_d,
        front_h: front_mm,
        door_top,
        ..
    } = cabinet.input;
    let panel =
        |part: &str, corner: [&str; 3], size: [&str; 3]| kit.panel(part, corner, size, None);
    let count = drawers as usize;
    let gap_mm = mm(DOOR_GAP);
    // The fronts below the top one, and the top one.
    // ponytail: `_drawer_height` set to 0 in Parameters gives fronts of 0 and the rebuild
    // fails; all alike, or 150 over doors, is chosen through Edit cabinet.
    let (front_h, top_h) = match edh {
        Some(edh) if doors > 0.0 => (edh.clone(), edh.clone()),
        Some(edh) => (
            format!(
                "(({eh} - {epl} - {}*{gap_mm} - {edh}) / {})",
                count + 1,
                count.max(2) - 1
            ),
            edh.clone(),
        ),
        None => {
            let equal = format!(
                "(({eh} - {epl} - {}*{gap_mm}) / {})",
                count + 1,
                count.max(1)
            );
            (equal.clone(), equal)
        }
    };
    let box_w = format!("({eiw} - {})", mm(2.0 * SLIDE));
    let box_d = format!("({eshelf_d} - {})", mm(BOX_BEHIND));
    let lift = format!("({et} + {})", mm(BOX_CLEAR));
    let (bt, back_y) = (mm(BOX_T), format!("-{gap_mm} + {box_d} - {}", mm(BOX_T)));
    for i in 0..count {
        let drawer = format!("Drawer {}", i + 1);
        let z = format!("{ez1} + {gap_mm} + ({front_h} + {gap_mm}) * {i}"); // front's lower edge
        let front_h = if i + 1 == count { &top_h } else { &front_h };
        let box_h = format!("({front_h} - 2*{lift})");
        let front_w = format!("{ew} - {}", mm(2.0 * DOOR_GAP));
        let y = format!("-{gap_mm} - {et}");
        kit.open(&drawer);
        panel(
            &format!("{drawer} Front"),
            [&gap_mm, &y, &z],
            [&front_w, et, front_h],
        );
        // Lying across, in the middle of the front unless the top or the bottom is asked for.
        let handle_z = match cabinet.input.handle_at {
            "top" => format!("{z} + {front_h} - {}", mm(HANDLE_EDGE)),
            "bottom" => format!("{z} + {}", mm(HANDLE_EDGE)),
            _ => format!("{z} + {front_h}/2"),
        };
        cabinet.handle(&format!("{drawer} Front"), &format!("{ew}/2"), &handle_z, 0);
        // The box touches the back of the front. Its sides run the full depth; front, back
        // and bottom sit between them.
        let (x, y, z) = (
            format!("{et} + {}", mm(SLIDE)),
            format!("-{gap_mm}"),
            format!("{z} + {lift}"),
        );
        let right = format!("{x} + {box_w} - {bt}");
        let inner_x = format!("{x} + {bt}");
        let inner_w = format!("{box_w} - {}", mm(2.0 * BOX_T));
        let inner_y = format!("{y} + {bt}");
        let inner_d = format!("{box_d} - {}", mm(2.0 * BOX_T));
        panel(
            &format!("{drawer} Left"),
            [&x, &y, &z],
            [&bt, &box_d, &box_h],
        );
        panel(
            &format!("{drawer} Right"),
            [&right, &y, &z],
            [&bt, &box_d, &box_h],
        );
        panel(
            &format!("{drawer} Box front"),
            [&inner_x, &y, &z],
            [&inner_w, &bt, &box_h],
        );
        panel(
            &format!("{drawer} Box back"),
            [&inner_x, &back_y, &z],
            [&inner_w, &bt, &box_h],
        );
        panel(
            &format!("{drawer} Bottom"),
            [&inner_x, &inner_y, &z],
            [&inner_w, &inner_d, &mm(BOX_BOTTOM)],
        );
        // The slide: straight out of the front, as far as four fifths of the box.
        let at = [
            0.0,
            -d / 2.0 - DOOR_GAP,
            door_top + DOOR_GAP + (front_mm + DOOR_GAP) * i as f64,
        ];
        let travel = ((shelf_d - BOX_BEHIND) * 0.8).round();
        kit.close(&drawer, None, "slider", "slide", at, [0, -1, 0], travel);
    }
}

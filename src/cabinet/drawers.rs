//! Drawers: an overlay front with a box behind it, a component of its own on a slider joint.

use super::input::Input;
use super::Cabinet;
use crate::fittings::DOOR_GAP;
use crate::kit::mm;

pub const SLIDE: f64 = 12.7; // ball-bearing drawer slide, each side
const BOX_T: f64 = 12.0; // drawer box sides, front and back
const BOX_BOTTOM: f64 = 6.0;
pub const BOX_LOWER: f64 = 40.0; // the box is this much lower than its front
const BOX_LIFT: f64 = 15.0; // and starts this far above the front's lower edge
const BOX_BEHIND: f64 = 10.0; // room behind the box

/// A stack of equal drawers: an overlay front, and behind it a box that runs on slides.
pub fn build(cabinet: &Cabinet) {
    let Cabinet {
        kit,
        ew,
        eh,
        et,
        epl,
        eiw,
        eshelf_d,
        ..
    } = cabinet;
    let Input {
        d,
        plinth,
        drawers,
        shelf_d,
        front_h: front_mm,
        ..
    } = cabinet.input;
    let panel =
        |part: &str, corner: [&str; 3], size: [&str; 3]| kit.panel(part, corner, size, None);
    let count = drawers as usize;
    let gap_mm = mm(DOOR_GAP);
    let front_h = format!(
        "(({eh} - {epl} - {}*{gap_mm}) / {})",
        count + 1,
        count.max(1)
    );
    let box_w = format!("({eiw} - {})", mm(2.0 * SLIDE));
    let box_d = format!("({eshelf_d} - {})", mm(BOX_BEHIND));
    let box_h = format!("({front_h} - {})", mm(BOX_LOWER));
    let (bt, back_y) = (mm(BOX_T), format!("-{gap_mm} + {box_d} - {}", mm(BOX_T)));
    for i in 0..count {
        let drawer = format!("Drawer {}", i + 1);
        let z = format!("{epl} + {gap_mm} + ({front_h} + {gap_mm}) * {i}"); // front's lower edge
        let front_w = format!("{ew} - {}", mm(2.0 * DOOR_GAP));
        let y = format!("-{gap_mm} - {et}");
        kit.open(&drawer);
        panel(
            &format!("{drawer} Front"),
            [&gap_mm, &y, &z],
            [&front_w, et, &front_h],
        );
        // In the middle of the front, lying across.
        cabinet.handle(
            &format!("{drawer} Front"),
            &format!("{ew}/2"),
            &format!("{z} + {front_h}/2"),
            0,
        );
        // The box touches the back of the front. Its sides run the full depth; front, back
        // and bottom sit between them.
        let (x, y, z) = (
            format!("{et} + {}", mm(SLIDE)),
            format!("-{gap_mm}"),
            format!("{z} + {}", mm(BOX_LIFT)),
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
            plinth + DOOR_GAP + (front_mm + DOOR_GAP) * i as f64,
        ];
        let travel = ((shelf_d - BOX_BEHIND) * 0.8).round();
        kit.close(&drawer, None, "slider", "slide", at, [0, -1, 0], travel);
    }
}

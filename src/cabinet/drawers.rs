//! Drawers: an overlay front with a box behind it, a component of its own on a slider joint.

use super::input::Input;
use super::Cabinet;
use crate::kit::mm;

pub const SLIDE: f64 = 12.7; // ball-bearing drawer slide, each side
const BOX_T: f64 = 12.0; // drawer box sides, front and back
const BOX_BOTTOM: f64 = 6.0;
// The box stays a panel thickness and this much inside its front's lower and upper edge: the
// lowest box then clears the cabinet's bottom, and the highest its top, by this and the gap.
pub const BOX_CLEAR: f64 = 8.0;
const BOX_BEHIND: f64 = 10.0; // room behind the box
const HANDLE_EDGE: f64 = 40.0; // a handle at the top or bottom: from the front's edge

/// A stack of equal drawers: a front, overlay or inset, and behind it a box that runs on slides.
pub fn build(cabinet: &Cabinet) {
    let Cabinet {
        kit,
        ew,
        eh,
        et,
        epl,
        eiw,
        eshelf_d,
        er,
        ein,
        efront,
        eback,
        ..
    } = cabinet;
    let Input {
        d,
        t,
        plinth,
        drawers,
        shelf_d,
        front_h: front_mm,
        inset,
        reveal,
        ..
    } = cabinet.input;
    let panel =
        |part: &str, corner: [&str; 3], size: [&str; 3]| kit.panel(part, corner, size, None);
    let count = drawers as usize;
    let gap_mm = er;
    // The fronts' room: the height above the plinth, less the panels round inset fronts.
    let room = format!("{eh} - {epl} - 2*{ein}");
    let front_h = format!("(({room} - {}*{gap_mm}) / {})", count + 1, count.max(1));
    let box_w = format!("({eiw} - {})", mm(2.0 * SLIDE));
    let box_d = format!("({eshelf_d} - {})", mm(BOX_BEHIND));
    // An overlay front laps the carcass by a panel; an inset one is already inside it.
    let lift = match inset {
        true => mm(BOX_CLEAR),
        false => format!("({et} + {})", mm(BOX_CLEAR)),
    };
    let box_h = format!("({front_h} - 2*{lift})");
    let (bt, back_y) = (mm(BOX_T), format!("{eback} + {box_d} - {}", mm(BOX_T)));
    for i in 0..count {
        let drawer = format!("Drawer {}", i + 1);
        // the front's lower edge
        let z = format!("{epl} + {ein} + {gap_mm} + ({front_h} + {gap_mm}) * {i}");
        let front_w = format!("{ew} - 2*{ein} - 2*{gap_mm}");
        let x0 = format!("{ein} + {gap_mm}");
        kit.open(&drawer);
        panel(
            &format!("{drawer} Front"),
            [&x0, efront, &z],
            [&front_w, et, &front_h],
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
            eback.clone(),
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
        let (o, back_mm) = if inset { (t, t) } else { (0.0, -reveal) };
        let at = [
            0.0,
            -d / 2.0 + back_mm,
            plinth + o + reveal + (front_mm + reveal) * i as f64,
        ];
        let travel = ((shelf_d - BOX_BEHIND) * 0.8).round();
        kit.close(&drawer, None, "slider", "slide", at, [0, -1, 0], travel);
    }
}

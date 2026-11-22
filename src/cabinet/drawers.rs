//! Drawers: an overlay front with a box behind it, a component of its own on a slider joint.

use super::input::Input;
use super::Cabinet;
use crate::fittings;
use crate::hardware;
use crate::kit::mm;

pub const SLIDE: f64 = 12.7; // ball-bearing drawer slide, each side
                             // The box stays a panel thickness and this much inside its front's lower and upper edge: the
                             // lowest box then clears the cabinet's bottom, and the highest its top, by this and the gap.
pub const BOX_CLEAR: f64 = 8.0;
pub const BOX_BEHIND: f64 = 10.0; // room behind the box
const HANDLE_EDGE: f64 = 40.0; // a handle at the top or bottom: from the front's edge

/// A stack of drawers: a front, overlay or inset, and behind it a box that runs on slides. They
/// are alike, or the top one is `drawer_height` and the rest share what is left, or beside
/// doors all are `drawer_height`. Under a sink the top front is a false front, fixed and with no
/// box, and the boxes below are half the inside depth deep, clear of the trap.
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
        edraw,
        edh,
        ..
    } = cabinet;
    let Input {
        d,
        t,
        drawers,
        front_h: front_mm,
        inset,
        reveal,
        mixed,
        drawer_z,
        kind,
        box_t,
        runner,
        ..
    } = cabinet.input;
    let sink = kind == "sink";
    let panel =
        |part: &str, corner: [&str; 3], size: [&str; 3]| kit.panel(part, corner, size, None);
    let count = drawers as usize;
    let gap_mm = er;
    // The fronts' room: the height above the plinth, less the panels round inset fronts.
    let room = format!("{eh} - {epl} - 2*{ein}");
    // The fronts below the top one, and the top one.
    // ponytail: `_drawer_height` set to 0 in Parameters gives fronts of 0 and the rebuild
    // fails; all alike, or 150 beside doors, is chosen through Edit cabinet.
    let (front_h, top_h) = match edh {
        Some(edh) if mixed => (edh.clone(), edh.clone()),
        Some(edh) => (
            format!(
                "(({room} - {}*{gap_mm} - {edh}) / {})",
                count + 1,
                count.max(2) - 1
            ),
            edh.clone(),
        ),
        None => {
            let equal = format!("(({room} - {}*{gap_mm}) / {})", count + 1, count.max(1));
            (equal.clone(), equal)
        }
    };
    // Wooden runners: the box stands `play` clear of each side, on a bottom wider than it that
    // runs in a groove `depth` deep in each side, `play` short of its end and round it.
    let wood = runner.map(|[depth, play]| {
        [
            kit.parameter("runner_depth", depth, "wooden runner groove depth"),
            kit.parameter("runner_play", play, "wooden runner play round the bottom"),
        ]
    });
    let (box_w, gap_x) = match &wood {
        Some([_, play]) => (format!("({eiw} - 2*{play})"), play.clone()),
        None => (format!("({eiw} - 2*HW_slide_t)"), "HW_slide_t".into()),
    };
    // ponytail: a sink base's boxes stop at half depth, before the trap; a U-cut round it
    // for deeper boxes is left to the user.
    let box_d = match sink {
        true => format!("({eshelf_d}/2)"),
        false => format!("({eshelf_d} - {})", mm(BOX_BEHIND)),
    };
    // An overlay front laps the carcass by a panel; an inset one is already inside it, but
    // beside doors it clears the rail as an overlay one does.
    let lift = match inset && !mixed {
        true => mm(BOX_CLEAR),
        false => format!("({et} + {})", mm(BOX_CLEAR)),
    };
    // The box's boards (sides, front and back) and its bottom: parameters while there are boxes.
    let [bt, bb] = match count > 1 || (count == 1 && !sink) {
        true => [
            kit.parameter("drawer_thickness", box_t[0], "drawer box board thickness"),
            kit.parameter("drawer_bottom", box_t[1], "drawer box bottom thickness"),
        ],
        false => return,
    };
    let back_y = format!("{eback} + {box_d} - {bt}");
    for i in 0..count {
        let drawer = format!("Drawer {}", i + 1);
        // the front's lower edge
        let z = format!("{edraw} + ({front_h} + {gap_mm}) * {i}");
        let front_h = if i + 1 == count { &top_h } else { &front_h };
        let box_h = format!("({front_h} - 2*{lift})");
        let front_w = format!("{ew} - 2*{ein} - 2*{gap_mm}");
        let x0 = format!("{ein} + {gap_mm}");
        // Lying across, in the middle of the front unless the top or the bottom is asked for.
        let handle_z = match cabinet.input.handle_at {
            "top" => format!("{z} + {front_h} - {}", mm(HANDLE_EDGE)),
            "bottom" => format!("{z} + {}", mm(HANDLE_EDGE)),
            _ => format!("{z} + {front_h}/2"),
        };
        if sink && i + 1 == count {
            panel("False front", [&x0, efront, &z], [&front_w, et, front_h]);
            cabinet.handle("False front", &format!("{ew}/2"), &handle_z, 0);
            break;
        }
        kit.open(&drawer);
        panel(
            &format!("{drawer} Front"),
            [&x0, efront, &z],
            [&front_w, et, front_h],
        );
        cabinet.handle(&format!("{drawer} Front"), &format!("{ew}/2"), &handle_z, 0);
        // The box touches the back of the front. Its sides run the full depth; front, back
        // and bottom sit between them.
        let (x, y, z) = (
            format!("{et} + {gap_x}"),
            eback.clone(),
            format!("{z} + {lift}"),
        );
        // On runners the bottom is under the box, and the box's boards stand on it.
        let (bz, box_h) = match &wood {
            Some(_) => (format!("{z} + {bb}"), format!("({box_h} - {bb})")),
            None => (z.clone(), box_h),
        };
        let right = format!("{x} + {box_w} - {bt}");
        let inner_x = format!("{x} + {bt}");
        let inner_w = format!("{box_w} - 2*{bt}");
        let inner_y = format!("{y} + {bt}");
        let inner_d = format!("{box_d} - 2*{bt}");
        panel(
            &format!("{drawer} Left"),
            [&x, &y, &bz],
            [&bt, &box_d, &box_h],
        );
        panel(
            &format!("{drawer} Right"),
            [&right, &y, &bz],
            [&bt, &box_d, &box_h],
        );
        panel(
            &format!("{drawer} Box front"),
            [&inner_x, &y, &bz],
            [&inner_w, &bt, &box_h],
        );
        panel(
            &format!("{drawer} Box back"),
            [&inner_x, &back_y, &bz],
            [&inner_w, &bt, &box_h],
        );
        // The slide (or runner): straight out of the front, as far as its type travels.
        let travel = cabinet.input.slide_type.1;
        let back_mm = if inset { t } else { -reveal };
        let at = [
            0.0,
            -d / 2.0 + back_mm,
            drawer_z + (front_mm + reveal) * i as f64,
        ];
        if let Some([depth, play]) = &wood {
            let out = format!("{et} - {depth} + {play}");
            let length = format!("{ew} - 2*({out})");
            panel(
                &format!("{drawer} Bottom"),
                [&out, &y, &z],
                [&length, &box_d, &bb],
            );
            kit.close(&drawer, None, "slider", "slide", at, [0, -1, 0], travel);
            // Its groove in each side, from the front to the back, in the cabinet's component:
            // named `<Side> runner <n>`, which the cut list notes.
            let (gz, gh) = (format!("{z} - {play}"), format!("{bb} + 2*{play}"));
            let gd = format!("{ein} + {eshelf_d} + 1 mm");
            let sides = [
                ("Side L", format!("{et} - {depth}")),
                ("Side R", format!("{ew} - {et}")),
            ];
            for (side, gx) in sides {
                let target = [format!("{} {side}", kit.name)];
                let part = format!("{side} runner {}", i + 1);
                kit.panel(&part, [&gx, "-1 mm", &gz], [depth, &gd, &gh], Some(&target));
            }
            continue;
        }
        panel(
            &format!("{drawer} Bottom"),
            [&inner_x, &inner_y, &z],
            [&inner_w, &inner_d, &bb],
        );
        // The slides' screw holes, in the middle of the box side's height: into each box side
        // from outside, `HW_setback` behind the box's front and the last whole `HW_pitch`
        // before `HW_setback` from the slide's back end; and into the carcass side beside it,
        // as far behind the front of the opening. The box's are named with the slide's
        // length, which the cut list's hardware reads.
        let slide_z = format!("{z} + {box_h}/2");
        let right_x = format!("{right} + {bt}");
        let halves = [
            ("Left", &x, 1, "Side L", et.clone(), -1),
            ("Right", &right_x, -1, "Side R", format!("{ew} - {et}"), 1),
        ];
        let (label, kind) = (cabinet.input.slide.1, cabinet.input.slide_type.0);
        let back = format!(" + floor(({label} - 2*HW_setback) / HW_pitch) * HW_pitch");
        let along = ["", back.as_str()];
        for (part, outer, into, ..) in &halves {
            for (k, along) in along.iter().enumerate() {
                let position = kit.at(outer, &format!("{y} + HW_setback{along}"), &slide_z);
                let (d, depth) = ("HW_box_screw_d", "HW_box_screw_depth");
                let what = format!("slide {} ({label}, {kind})", k + 1);
                fittings::hole(
                    kit,
                    &format!("{drawer} {part}"),
                    position,
                    [*into, 0, 0],
                    d,
                    depth,
                    &what,
                );
            }
        }
        // The slides' models (`hardware::MEMBERS`): the cabinet member against each carcass
        // side, in the cabinet's component; the drawer member against each box side, in the
        // drawer's, so it moves with the drawer; and on a 3-section slide the middle member
        // half way between, also in the drawer's.
        // ponytail: the middle member moves as far as the drawer, not half as far: that would
        // take a joint driven by the slide's, which the app does not have.
        let [(_, cabinet_t, _), _, (_, drawer_t, _)] = hardware::MEMBERS;
        let (half_c, half_d) = (mm(cabinet_t / 2.0), mm(drawer_t / 2.0));
        let middle = format!("HW_slide_t/2 + {}", mm((cabinet_t - drawer_t) / 2.0));
        let sides = [
            (et.to_string(), x.to_string(), 1),
            (format!("{ew} - {et}"), right_x.clone(), -1),
        ];
        let component = format!("{} {drawer}", kit.name);
        let upright = hardware::upright([0, -1, 0]);
        for (carcass, box_face, out) in &sides {
            let sign = if *out > 0 { "+" } else { "-" };
            let (back, to) = (if *out > 0 { "-" } else { "+" }, kit.name.as_str());
            let models = [
                (
                    hardware::SLIDE_CABINET,
                    format!("{carcass} {sign} {half_c}"),
                    to,
                ),
                (
                    hardware::SLIDE_DRAWER,
                    format!("{box_face} {back} {half_d}"),
                    &component,
                ),
                (
                    hardware::SLIDE_MIDDLE,
                    format!("{carcass} {sign} ({middle})"),
                    &component,
                ),
            ];
            let two = kind == "partial extension";
            for (model, along_x, parent) in models.iter().take(if two { 2 } else { 3 }) {
                let at = kit.at(along_x, &y, &slide_z);
                let model = format!("{model} {label}");
                hardware::copy(kit, &model, parent, at, upright, None);
            }
        }
        kit.close(&drawer, None, "slider", "slide", at, [0, -1, 0], travel);
        for (_, _, _, side, inner, out) in &halves {
            for (k, along) in along.iter().enumerate() {
                let position = kit.at(inner, &format!("{ein} + HW_setback{along}"), &slide_z);
                let what = format!("{drawer} slide {}", k + 1);
                fittings::hole(
                    kit,
                    side,
                    position,
                    [*out, 0, 0],
                    "HW_screw_d",
                    &hardware::screw_depth(et),
                    &what,
                );
            }
        }
    }
}

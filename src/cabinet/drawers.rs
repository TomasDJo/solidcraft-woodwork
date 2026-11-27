//! Drawers: an overlay front with a box behind it, a component of its own on a slider joint.

use super::input::Input;
use super::Cabinet;
use crate::fittings;
use crate::hardware;
use crate::kit::mm;
use serde_json::json;

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
        box_h: heights,
        runner,
        strip,
        under,
        box_fingers: fingers,
        integrated,
        ..
    } = cabinet.input;
    let sink = kind == "sink";
    let panel =
        |part: &str, corner: [&str; 3], size: [&str; 3]| kit.panel(part, corner, size, None);
    let count = drawers as usize;
    // Shaker fronts only where every front is tall and wide enough for its frame, and not
    // integrated: the width less the panels round inset fronts and the reveals.
    let framed = cabinet.input.framed_drawers;
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
    // On strip runners (#146) there is no groove: the bottom runs on a strip on each side.
    let strip = strip.map(|[st, sh]| {
        [
            kit.parameter("strip_thickness", st, "drawer runner strip thickness"),
            kit.parameter("strip_height", sh, "drawer runner strip height"),
        ]
    });
    let wood = runner.map(|[depth, play]| {
        [
            match strip {
                Some(_) => String::new(),
                None => kit.parameter("runner_depth", depth, "wooden runner groove depth"),
            },
            kit.parameter("runner_play", play, "wooden runner play round the bottom"),
        ]
    });
    // The box's boards (sides, front and back) and its bottom: parameters while there are boxes.
    let [bt, bb] = match count > 1 || (count == 1 && !sink) {
        true => [
            kit.parameter("drawer_thickness", box_t[0], "drawer box board thickness"),
            kit.parameter("drawer_bottom", box_t[1], "drawer box bottom thickness"),
        ],
        false => return,
    };
    // The bottom's tab (#181): a parameter while drawers have one.
    let tab_p = (cabinet.input.handles == "tab")
        .then(|| kit.parameter("tab_width", cabinet.input.tab_w, "drawer bottom tab width"));
    // Lower sides and back (#221): parameters only when asked, each capped by what it stands
    // beside, so a taller value in Parameters gives the full height instead of a box that
    // pokes out.
    let [side_p, back_p] = [
        ("drawer_side_height", heights[0], "drawer box side height"),
        ("drawer_back_height", heights[1], "drawer box back height"),
    ]
    .map(|(key, mm_value, comment)| {
        let used = mm_value > 0.0;
        used.then(|| kit.parameter(key, mm_value, comment))
    });
    // Undermount slides (Blum's rule): the box 42 mm narrower inside than the opening, and as
    // long as the slide.
    let (box_w, gap_x) = match (&wood, under) {
        (Some([_, play]), _) => match &strip {
            Some([st, _]) => (
                format!("({eiw} - 2*({st} + {play}))"),
                format!("({st} + {play})"),
            ),
            None => (format!("({eiw} - 2*{play})"), play.clone()),
        },
        (_, true) => (
            format!("({eiw} - 42 mm + 2*{bt})"),
            format!("(21 mm - {bt})"),
        ),
        _ => (format!("({eiw} - 2*HW_slide_t)"), "HW_slide_t".into()),
    };
    // ponytail: a sink base's boxes stop at half depth, before the trap; a U-cut round it
    // for deeper boxes is left to the user.
    let label = cabinet.input.slide.1;
    let box_d = match (sink, under) {
        (_, true) => label.to_string(),
        (true, _) => format!("({eshelf_d}/2)"),
        (false, _) => format!("({eshelf_d} - {})", mm(BOX_BEHIND)),
    };
    // An overlay front laps the carcass by a panel; an inset one is already inside it, but
    // beside doors it clears the rail as an overlay one does.
    let lift = match inset && !mixed {
        true => mm(BOX_CLEAR),
        false => format!("({et} + {})", mm(BOX_CLEAR)),
    };
    let back_y = format!("{eback} + {box_d} - {bt}");
    for i in 0..count {
        let drawer = format!("Drawer {}", i + 1);
        // the front's lower edge
        let z = format!("{edraw} + ({front_h} + {gap_mm}) * {i}");
        let front_h = if i + 1 == count { &top_h } else { &front_h };
        // A groove or grip edge on the top edge, or the bottom one when asked (#230); a grip
        // edge's front stops the grip gap short of it, and its box with it.
        let top = cabinet.input.handle_at != "bottom";
        let (z, front_h) = cabinet.gripped(&z, front_h, top);
        let front_h = &front_h;
        let box_h = format!("({front_h} - 2*{lift})");
        let front_w = format!("{ew} - 2*{ein} - 2*{gap_mm}");
        let x0 = format!("{ein} + {gap_mm}");
        // Lying across, in the middle of the front unless the top or the bottom is asked for.
        // A glass front's handle goes in the middle of a rail, clear of the pane: the bottom one
        // when asked, else the top one (auto too; the middle is refused, `input`).
        let rail = (cabinet.eframe.as_ref())
            .filter(|_| framed && cabinet.input.glass)
            .map(|[fw, _]| format!("{fw}/2"));
        let handle_z = match (cabinet.input.handle_at, rail) {
            ("bottom", Some(half)) => format!("{z} + {half}"),
            (_, Some(half)) => format!("{z} + {front_h} - {half}"),
            ("top", None) => format!("{z} + {front_h} - {}", mm(HANDLE_EDGE)),
            ("bottom", None) => format!("{z} + {}", mm(HANDLE_EDGE)),
            _ => format!("{z} + {front_h}/2"),
        };
        if sink && i + 1 == count {
            cabinet.front(
                "False front",
                [&x0, efront, &z],
                [&front_w, et, front_h],
                true,
                framed,
            );
            cabinet.handle("False front", &format!("{ew}/2"), &handle_z, 0);
            cabinet.grooves("False front", [&x0, efront, &z], [&front_w, et, front_h]);
            cabinet.grip(
                "False front",
                [&x0, efront, &z],
                [&front_w, et, front_h],
                top,
            );
            break;
        }
        kit.open(&drawer);
        // Integrated (#163): no front of its own, the box's front board takes its size and place.
        let front = match integrated {
            true => format!("{drawer} Box front"),
            false => format!("{drawer} Front"),
        };
        // An integrated front is exactly as wide as the box (#223): reaching past its sides,
        // their fingers would end in closed mortises in its back. The slide gap shows.
        let (x0, front_w) = match integrated {
            true => (format!("{et} + {gap_x}"), box_w.clone()),
            false => (x0, front_w),
        };
        cabinet.front(
            &front,
            [&x0, efront, &z],
            [&front_w, et, front_h],
            true,
            framed,
        );
        cabinet.handle(&front, &format!("{ew}/2"), &handle_z, 0);
        cabinet.grooves(&front, [&x0, efront, &z], [&front_w, et, front_h]);
        cabinet.grip(&front, [&x0, efront, &z], [&front_w, et, front_h], top);
        let front_z = z.clone();
        // The box touches the back of the front. Its sides run the full depth; front, back
        // and bottom sit between them.
        let (x, y, z) = (
            format!("{et} + {gap_x}"),
            eback.clone(),
            format!("{z} + {lift}"),
        );
        // On strips the bottom goes up by as much of the strip as `BOX_CLEAR` leaves under it,
        // so the strip stays clear of the carcass bottom, rail or drawer below; the box's top
        // stays where it was.
        let (z, box_h) = match &strip {
            Some([_, sh]) => {
                let rise = format!("max(0 mm, {sh} - {})", mm(BOX_CLEAR));
                (format!("{z} + {rise}"), format!("({box_h} - {rise})"))
            }
            None => (z, box_h),
        };
        // On runners the bottom is under the box, and the box's boards stand on it.
        let (bz, box_h) = match &wood {
            Some(_) => (format!("{z} + {bb}"), format!("({box_h} - {bb})")),
            None => (z.clone(), box_h),
        };
        // the sides, the back as high as them unless asked; the front stays full height
        let side_h = match &side_p {
            Some(p) => format!("min({box_h}, {p})"),
            None => box_h.clone(),
        };
        // finger-jointed too: its corners' fingers are laid out over its own height (below)
        let back_h = match &back_p {
            Some(p) => format!("min({side_h}, {p})"),
            None => side_h.clone(),
        };
        let right = format!("{x} + {box_w} - {bt}");
        let inner_x = format!("{x} + {bt}");
        let inner_w = format!("{box_w} - 2*{bt}");
        // An integrated front stands before the box, so the bottom starts right behind it.
        let (inner_y, inner_d) = match integrated {
            true => (y.clone(), format!("{box_d} - {bt}")),
            false => (format!("{y} + {bt}"), format!("{box_d} - 2*{bt}")),
        };
        // Bottom tab (#181): the bottom, its underside at `tz` and its front edge at `ty`, runs
        // forward under the front to `TAB_OUT` past its face, `tab_width` wide (never wider than
        // the box inside), its two front corners rounded. The front is cut away from its lower
        // edge up to the bottom's top where the tab passes, and scooped above it; a box front
        // the bottom stands behind is slotted for it.
        let tab = |tz: &str, ty: &str, through_box: bool| {
            let Some(tab_p) = &tab_p else { return };
            let w = format!("min({tab_p}, {box_w} - 2*{bt})");
            let r = mm(fittings::TAB_R);
            let tx = format!("{ew}/2 - {w}/2");
            let ty0 = format!("{efront} - {}", mm(fittings::TAB_OUT));
            let notch = format!("({tz}) + {bb} - ({front_z}) + 1 mm");
            kit.panel(
                &format!("{front} tab notch"),
                [
                    &tx,
                    &format!("{efront} - 1 mm"),
                    &format!("{front_z} - 1 mm"),
                ],
                [&w, &format!("{et} + 2 mm"), &notch],
                Some(&kit.targets(&front)),
            );
            if through_box {
                let target = [format!("{} {drawer} Box front", kit.name)];
                kit.panel(
                    &format!("{drawer} Box front tab slot"),
                    [&tx, &format!("{y} - 1 mm"), tz],
                    [&w, &format!("{bt} + 2 mm"), &bb],
                    Some(&target),
                );
            }
            let bottom = [format!("{} {drawer} Bottom", kit.name)];
            let name = format!("{} {drawer} Bottom tab", kit.name);
            let boxes = [
                (
                    tx.clone(),
                    format!("{ty0} + {r}"),
                    w.clone(),
                    format!("({ty}) - ({ty0}) - {r} + 1 mm"),
                    "1",
                ),
                (
                    format!("{tx} + {r}"),
                    ty0.clone(),
                    format!("{w} - 2*{r}"),
                    format!("2*{r}"),
                    "2",
                ),
            ];
            for (bx, by, length, width, k) in boxes {
                kit.push((
                    "solid.box",
                    json!({
                        "corner": kit.at(&bx, &by, tz), "length": length, "width": width,
                        "height": bb, "operation": "join", "targets": bottom, "name": format!("{name} {k}"),
                    }),
                ));
            }
            for (k, cx) in [format!("{tx} + {r}"), format!("{tx} + {w} - {r}")]
                .into_iter()
                .enumerate()
            {
                kit.push((
                    "solid.cylinder",
                    json!({
                        "base": kit.at(&cx, &format!("{ty0} + {r}"), tz), "axis": [0, 0, 1],
                        "diameter": format!("2*{r}"), "height": bb, "operation": "join",
                        "targets": bottom, "name": format!("{name} corner {}", k + 1),
                    }),
                ));
            }
            // its axis half its diameter over the tab: the half round rises from the tab's top
            let d = mm(fittings::GROOVE_D);
            let scoop = kit.at(&tx, efront, &format!("{tz} + {bb} + {d}/2"));
            fittings::hole(kit, &front, scoop, [1, 0, 0], &d, &w, "tab scoop");
        };
        // Finger-jointed into an integrated front, the sides run forward through it.
        // ponytail: an inset front on runners with less play than reveal leaves the fingers
        // proud of the front's edge by the difference.
        let (side_y, side_d) = match integrated && fingers {
            true => (efront.clone(), format!("({box_d} + {et})")),
            false => (y.clone(), box_d.clone()),
        };
        panel(
            &format!("{drawer} Left"),
            [&x, &side_y, &bz],
            [&bt, &side_d, &side_h],
        );
        panel(
            &format!("{drawer} Right"),
            [&right, &side_y, &bz],
            [&bt, &side_d, &side_h],
        );
        // With finger joints (#131) the box's front and back run through the sides to the
        // outside, as wide as the box.
        let (end_x, end_w) = match fingers {
            true => (x.clone(), box_w.clone()),
            false => (inner_x.clone(), inner_w.clone()),
        };
        if !integrated {
            panel(
                &format!("{drawer} Box front"),
                [&end_x, &y, &bz],
                [&end_w, &bt, &box_h],
            );
        }
        panel(
            &format!("{drawer} Box back"),
            [&end_x, &back_y, &bz],
            [&end_w, &bt, &back_h],
        );
        if fingers {
            // The four upright corners: an odd count of fingers as wide as the boards are
            // thick, the sides keeping the outer ones; one box cut per board at each corner and
            // two patterns up the box, shared by the four (as `joint::finger` does the shell).
            // The front corners over the sides' height, the front above them left whole; the
            // back's over its own (#221), lower when Drawer back height asks, the sides whole
            // above it. Each height its own pair of rows.
            let front_t = if integrated { et.clone() } else { bt.clone() };
            let mut runs: Vec<(String, Vec<String>, Vec<String>)> = Vec::new();
            for (side, sx) in [("Left", x.clone()), ("Right", right.clone())] {
                let boards = [
                    ("Box front", side_y.clone(), front_t.clone(), side_h.clone()),
                    ("Box back", back_y.clone(), bt.clone(), back_h.clone()),
                ];
                for (end, ey, deep, run) in boards {
                    let w = format!("(({run}) / (2*floor(({run}) / (2*{bt})) + 1))");
                    let at = match runs.iter().position(|r| r.0 == run) {
                        Some(at) => at,
                        None => {
                            runs.push((run.clone(), Vec::new(), Vec::new()));
                            runs.len() - 1
                        }
                    };
                    for (board, other, first, is_side) in
                        [(side, end, w.clone(), true), (end, side, mm(0.0), false)]
                    {
                        let target = [format!("{} {drawer} {board}", kit.name)];
                        let part = format!("{drawer} {board} finger {other}");
                        let z0 = format!("{bz} + {first}");
                        kit.panel(&part, [&sx, &ey, &z0], [&bt, &deep, &w], Some(&target));
                        let list = if is_side {
                            &mut runs[at].1
                        } else {
                            &mut runs[at].2
                        };
                        list.push(format!("{} {part}", kit.name));
                    }
                }
            }
            let mut k = 0;
            for (run, sides, ends) in runs {
                let half = format!("floor(({run}) / (2*{bt}))");
                let w = format!("(({run}) / (2*{half} + 1))");
                for (features, count) in [(sides, half.clone()), (ends, format!("({half} + 1)"))] {
                    k += 1;
                    kit.push((
                        "solid.pattern.rectangular",
                        json!({
                            "features": features, "name": format!("{} {drawer} fingers {k}", kit.name),
                            "dir1": [0, 0, 1], "count1": count, "spacing1": format!("2*{w}"),
                        }),
                    ));
                }
            }
        }
        // The slide (or runner): straight out of the front, as far as its type travels.
        let travel = cabinet.input.slide_type.1;
        let back_mm = if inset { t } else { -reveal };
        let at = [
            0.0,
            -d / 2.0 + back_mm,
            drawer_z + (front_mm + reveal) * i as f64,
        ];
        if under {
            // The bottom raised for the runners, the back notched over them at each lower
            // corner (35 mm from the box's outside), the runners' rear hooks' holes in the
            // back (the left one named as a slide's first hole, which the cut list counts),
            // and a runner model under each side.
            let lift_b = format!("{z} + {}", mm(hardware::UNDER_RECESS));
            panel(
                &format!("{drawer} Bottom"),
                [&inner_x, &inner_y, &lift_b],
                [&inner_w, &inner_d, &bb],
            );
            tab(&lift_b, &inner_y, !integrated);
            let back = format!("{drawer} Box back");
            let target = [format!("{} {back}", kit.name)];
            let (y0, hook_z) = (format!("{back_y} - 1 mm"), format!("{lift_b} + 7 mm"));
            let kind = cabinet.input.slide_type.0;
            let ends = [
                (x.clone(), format!("{x} + 17.5 mm"), "Left"),
                (
                    format!("{x} + {box_w} - 35 mm"),
                    format!("{x} + {box_w} - 17.5 mm"),
                    "Right",
                ),
            ];
            for (k, (notch_x, hook_x, side)) in ends.iter().enumerate() {
                kit.panel(
                    &format!("{back} notch {}", k + 1),
                    [notch_x, &y0, &format!("{z} - 1 mm")],
                    [
                        "35 mm",
                        &format!("{bt} + 2 mm"),
                        &format!("{} + 1 mm", mm(hardware::UNDER_RECESS)),
                    ],
                    Some(&target),
                );
                let at = kit.at(hook_x, &format!("{back_y} + {bt}"), &hook_z);
                let what = format!("{side} slide 1 ({label}, {kind})");
                fittings::hole(kit, &back, at, [0, -1, 0], "6 mm", "10 mm", &what);
                let runner_x = match k {
                    0 => format!("{inner_x} + {}", mm(hardware::UNDER_SIZE[0] / 2.0)),
                    _ => format!("{right} - {}", mm(hardware::UNDER_SIZE[0] / 2.0)),
                };
                let model = format!("{} {label}", hardware::SLIDE_UNDER);
                let component = format!("{} {drawer}", kit.name);
                let upright = hardware::upright([0, -1, 0]);
                hardware::copy(
                    kit,
                    &model,
                    &component,
                    kit.at(&runner_x, &y, &z),
                    upright,
                    None,
                );
            }
            kit.close(&drawer, None, "slider", "slide", at, [0, -1, 0], travel);
            continue;
        }
        if let Some([depth, play]) = &wood {
            let out = match strip {
                Some(_) => format!("{et} + {play}"),
                None => format!("{et} - {depth} + {play}"),
            };
            let length = format!("{ew} - 2*({out})");
            panel(
                &format!("{drawer} Bottom"),
                [&out, &y, &z],
                [&length, &box_d, &bb],
            );
            tab(&z, &y, false);
            kit.close(&drawer, None, "slider", "slide", at, [0, -1, 0], travel);
            // Its strip on each side, under the bottom from the front of the opening to the
            // back, in the cabinet's component: `<Side> strip <n>`, a board of the cut list.
            // ponytail: the strips' screw holes are not drilled.
            if let Some([st, sh]) = &strip {
                let sides = [
                    ("Side L", et.clone()),
                    ("Side R", format!("{ew} - {et} - {st}")),
                ];
                for (side, sx) in sides {
                    let part = format!("{side} strip {}", i + 1);
                    let sz = format!("{z} - {sh}");
                    panel(&part, [&sx, ein, &sz], [st, eshelf_d, sh]);
                }
                continue;
            }
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
        tab(&z, &inner_y, !integrated);
        // The slides' screw holes, in the middle of the box side's height: into each box side
        // from outside, `HW_setback` behind the box's front and the last whole `HW_pitch`
        // before `HW_setback` from the slide's back end; and into the carcass side beside it,
        // as far behind the front of the opening. The box's are named with the slide's
        // length, which the cut list's hardware reads.
        let slide_z = format!("{z} + {side_h}/2");
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

//! Mobile base (#101): one frame on locking casters under a group of cabinets, for a workbench
//! that rolls. Boards on edge round the cabinets' joint footprint, a cross board under each
//! place where two cabinets meet, a block in each corner (and two in the middle of a long
//! frame) with a caster under it. Numeric, at world coordinates, like the worktop: `lib.rs`
//! measures the cabinets, makes the frame, and stands the cabinets on it.

use crate::fittings;
use crate::kit::{check, mm, Kit};
use crate::Commands;
use serde_json::{json, Value};

/// A caster block's side, in mm: the caster's 80 mm plate fits under it.
const BLOCK: f64 = 90.0;
/// Past this length the frame gets a pair of casters in its middle too.
const LONG: f64 = 1500.0;

/// The plan boxes `[x0, y0, x1, y1]` of `cabinets` (their sides and backs seen from above, as
/// the worktop reads them) and each one's lowest point of any of its bodies: where it stands
/// now. `measure` is `inspect.measure`'s reply with those cabinets' bodies.
pub fn plans(measure: &Value, cabinets: &[String]) -> Result<(Vec<[f64; 4]>, Vec<f64>), String> {
    let (mut plans, mut floors) = (Vec::new(), Vec::new());
    for name in cabinets {
        let mut floor = f64::INFINITY;
        let mut box_ = [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ];
        for body in measure["bodies"].as_array().into_iter().flatten() {
            let Some(part) = (body["name"].as_str())
                .and_then(|n| n.strip_prefix(name.as_str()))
                .and_then(|p| p.strip_prefix(' '))
            else {
                continue;
            };
            let at = |end: &str, i: usize| body["bbox"][end][i].as_f64().unwrap_or(f64::NAN);
            floor = floor.min(at("min", 2));
            if ["Side L", "Side R", "Back", "Back return"].contains(&part) {
                box_ = [
                    box_[0].min(at("min", 0)),
                    box_[1].min(at("min", 1)),
                    box_[2].max(at("max", 0)),
                    box_[3].max(at("max", 1)),
                ];
            }
        }
        check(
            box_[0].is_finite(),
            &format!("`{name}` has no sides to stand on a base"),
        )?;
        plans.push(box_);
        floors.push(floor);
    }
    check(
        !plans.is_empty(),
        "pick a board of each cabinet to stand on the base",
    )?;
    Ok((plans, floors))
}

/// A timber frame's section `(thickness, width)` in mm (`frame_material: timber`): the timber's
/// thickness is the frame's across, its width the frame's height, standing on edge. 48 x 98 by
/// default (Norway's common stud). `None` for a frame of boards.
pub fn timber(p: &Value) -> Result<Option<(f64, f64)>, String> {
    if p["frame_material"] != "timber" {
        return Ok(None);
    }
    let t = p["timber_thickness"].as_f64().unwrap_or(48.0);
    let w = p["timber_width"].as_f64().unwrap_or(98.0);
    check(
        (20.0..=150.0).contains(&t),
        "timber thickness must be 20 to 150 mm",
    )?;
    check(
        (30.0..=300.0).contains(&w),
        "timber width must be 30 to 300 mm",
    )?;
    Ok(Some((t, w)))
}

/// The frame's commands and summary for the cabinets' `plans`: Caster height (the frame's
/// underside above the floor), Frame height and Thickness (its boards on edge). The summary's
/// `lift` is how high the cabinets stand, the frame's top.
pub fn build(p: &Value, plans: &[[f64; 4]]) -> Result<(Commands, Value), String> {
    let mut p = p.clone();
    if p.get("name").is_none() {
        p["name"] = json!("Base");
    }
    let kit = Kit::new(&p)?;
    let caster = kit.size(&p, "caster_height", 100.0)?;
    // mm only (no Units field), so the dialog's bounds are these
    check(
        (fittings::CASTER_MIN..=300.0).contains(&caster),
        "caster height must be 50 to 300 mm",
    )?;
    let (t, h) = match timber(&p)? {
        Some(section) => section,
        None => {
            let h = kit.size(&p, "frame_height", 70.0)?;
            let t = kit.size(&p, "thickness", 18.0)?;
            check(
                (30.0..=300.0).contains(&h),
                "frame height must be 30 to 300 mm",
            )?;
            check((3.0..=50.0).contains(&t), "thickness must be 3 to 50 mm")?;
            (t, h)
        }
    };
    let [x0, y0, x1, y1] = plans.iter().fold(
        [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ],
        |a, b| {
            [
                a[0].min(b[0]),
                a[1].min(b[1]),
                a[2].max(b[2]),
                a[3].max(b[3]),
            ]
        },
    );
    let (w, d) = (x1 - x0, y1 - y0);
    check(
        w.min(d) >= 2.0 * (t + BLOCK),
        "the cabinets are too small for a base: its corner blocks would meet",
    )?;
    // The frame's long way (0: along X), and its short way.
    let long = usize::from(d > w);
    let (lo, hi) = ([x0, y0][long], [x1, y1][long]);
    let (slo, shi) = ([x0, y0][1 - long], [x1, y1][1 - long]);
    // A box given along the long way and the short way, as a board of the frame.
    let board = |part: &str, a: f64, b: f64, la: f64, lb: f64| {
        let (corner, size) = match long {
            0 => ([a, b], [la, lb]),
            _ => ([b, a], [lb, la]),
        };
        let at = [mm(corner[0]), mm(corner[1]), mm(caster)];
        let size = [mm(size[0]), mm(size[1]), mm(h)];
        kit.panel(
            part,
            [&at[0], &at[1], &at[2]],
            [&size[0], &size[1], &size[2]],
            None,
        );
    };
    let (len, short) = (hi - lo, shi - slo);
    board("Frame front", lo, slo, len, t);
    board("Frame back", lo, shi - t, len, t);
    board("Frame left", lo, slo + t, t, short - 2.0 * t);
    board("Frame right", hi - t, slo + t, t, short - 2.0 * t);
    // A cross board where two cabinets meet, centred on the joint: across the frame where they
    // meet along its long way, along it where they stand side by side across it (a narrow run,
    // or cabinets back to back). One along the frame runs only under the cabinets with that edge,
    // in pieces between the boards across it, so no two boards overlap (#216).
    let inside = |axis: usize, from: f64, to: f64| {
        let mut at: Vec<f64> = (plans.iter())
            .flat_map(|p| [p[axis], p[axis + 2]])
            .filter(|&j| j > from + t + BLOCK && j < to - t - BLOCK)
            .collect();
        at.sort_by(f64::total_cmp);
        at.dedup_by(|a, b| (*a - *b).abs() < 1.0);
        at
    };
    let joints = inside(long, lo, hi);
    let mut n = 0;
    for j in &joints {
        n += 1;
        board(
            &format!("Frame cross {n}"),
            j - t / 2.0,
            slo + t,
            t,
            short - 2.0 * t,
        );
    }
    for j in inside(1 - long, slo, shi) {
        let edged = plans
            .iter()
            .filter(|p| (p[1 - long] - j).abs() < 1.0 || (p[3 - long] - j).abs() < 1.0);
        let stops: Vec<f64> = joints.iter().map(|c| c - t / 2.0).collect();
        for (a, b) in pieces(
            edged.map(|p| (p[long], p[long + 2])),
            lo + t,
            hi - t,
            &stops,
            t,
        ) {
            n += 1;
            board(&format!("Frame cross {n}"), a, j - t / 2.0, b - a, t);
        }
    }
    // Blocks in the corners, and on a long frame two more by the middle (beside the cross
    // board nearest it), each with a caster under its middle.
    let mut along = vec![lo + t, hi - t - BLOCK];
    if len > LONG {
        let middle = (lo + hi) / 2.0;
        let nearest = joints
            .iter()
            .min_by(|a, b| (*a - middle).abs().total_cmp(&(*b - middle).abs()));
        along.push(match nearest {
            Some(j) if (j - middle).abs() < len / 4.0 => j + t / 2.0,
            _ => middle - BLOCK / 2.0,
        });
    }
    let mut n = 0;
    for a in along {
        for b in [slo + t, shi - t - BLOCK] {
            n += 1;
            let part = format!("Block {n}");
            board(&part, a, b, BLOCK, BLOCK);
            let (ca, cb) = (a + BLOCK / 2.0, b + BLOCK / 2.0);
            let (x, y) = if long == 0 { (ca, cb) } else { (cb, ca) };
            fittings::caster(&kit, n, &mm(x), &mm(y), &mm(caster), &mm(h), &part);
        }
    }
    let (commands, mut summary) = kit.finish(vec![], [w, caster + h, d]);
    summary["lift"] = json!(caster + h);
    summary["casters"] = json!(n);
    Ok((commands, summary))
}

/// The stretches `(from, to)` a cross board covers: the union of `spans`, within `lo..hi`, less a
/// board `t` thick starting at each of `stops`. Stretches shorter than 1 mm are dropped.
fn pieces(
    spans: impl Iterator<Item = (f64, f64)>,
    lo: f64,
    hi: f64,
    stops: &[f64],
    t: f64,
) -> Vec<(f64, f64)> {
    let mut cuts: Vec<(f64, f64)> = spans
        .map(|(a, b)| (a.max(lo), b.min(hi)))
        .filter(|(a, b)| b > a)
        .collect();
    cuts.sort_by(|x, y| x.0.total_cmp(&y.0));
    let mut out: Vec<(f64, f64)> = Vec::new();
    for (a, b) in cuts {
        match out.last_mut() {
            Some(last) if a <= last.1 + 1.0 => last.1 = last.1.max(b),
            _ => out.push((a, b)),
        }
    }
    for &s in stops {
        out = (out.into_iter())
            .flat_map(|(a, b)| [(a, b.min(s)), (a.max(s + t), b)])
            .filter(|(a, b)| b - a >= 1.0)
            .collect();
    }
    out
}

/// A connector bolt's hole through two touching sides, in mm: one size for M6 bolts.
pub const BOLT_D: f64 = 9.0;

/// The connector bolt holes (#198) where a side of one of `cabinets` touches a side of another:
/// each the two sides in name order (the first gets the bolt the cut list counts), the hole's
/// number in that pair, 1 to 4, and its box in the world through both. None of it depends on
/// the order the cabinets are picked in, so a run again finds the same holes by name. Two rows of two, at a quarter and three quarters of where they touch,
/// across and up. `measure` is `inspect.measure`'s reply, the cabinets where they stand.
pub fn bolts(measure: &Value, cabinets: &[String]) -> Vec<([String; 2], usize, [[f64; 3]; 2])> {
    let all = measure["bodies"].as_array().cloned().unwrap_or_default();
    let sides: Vec<(usize, String, [[f64; 3]; 2])> = (cabinets.iter().enumerate())
        .flat_map(|(i, c)| ["Side L", "Side R"].map(|s| (i, format!("{c} {s}"))))
        .filter_map(|(i, side)| {
            let b = all.iter().find(|b| b["name"] == side.as_str())?;
            let at = |end: &str| [0, 1, 2].map(|k| b["bbox"][end][k].as_f64().unwrap_or(f64::NAN));
            Some((i, side, [at("min"), at("max")]))
        })
        .collect();
    let mut holes = Vec::new();
    for (i, sa, a) in &sides {
        for (j, sb, b) in &sides {
            if i >= j {
                continue;
            }
            for k in 0..2 {
                let touch = (a[1][k] - b[0][k]).abs() < 0.5 || (b[1][k] - a[0][k]).abs() < 0.5;
                let span = |axis: usize| (a[0][axis].max(b[0][axis]), a[1][axis].min(b[1][axis]));
                let ((o0, o1), (z0, z1)) = (span(1 - k), span(2));
                if !touch || o1 - o0 < 4.0 * BOLT_D || z1 - z0 < 4.0 * BOLT_D {
                    continue;
                }
                let pair = match sa < sb {
                    true => [sa.clone(), sb.clone()],
                    false => [sb.clone(), sa.clone()],
                };
                for f in [0.25, 0.75] {
                    for g in [0.25, 0.75] {
                        let (o, z) = (o0 + (o1 - o0) * f, z0 + (z1 - z0) * g);
                        let mut hole = [[0.0; 3]; 2];
                        hole[0][k] = a[0][k].min(b[0][k]);
                        hole[1][k] = a[1][k].max(b[1][k]);
                        for (axis, at) in [(1 - k, o), (2, z)] {
                            hole[0][axis] = at - BOLT_D / 2.0;
                            hole[1][axis] = at + BOLT_D / 2.0;
                        }
                        holes.push((pair.clone(), holes.len() % 4 + 1, hole));
                    }
                }
            }
        }
    }
    holes
}

/// A screw's clearance hole down through a cabinet's bottom into the frame, in mm: 4 x 50 mm
/// screws.
pub const SCREW_D: f64 = 4.5;

/// The screw holes (#198) down through each of `cabinets`' `Bottom` (and `Bottom return`) into a board of the base
/// `name`'s frame on which it rests: each the bottom, the board, the hole's number on that board,
/// 1 or 2, and its box in the world through the bottom. Two on each board that runs under the
/// bottom at least 100 mm, at a quarter and three quarters of that, on the board's middle line.
/// `measure` has the cabinets' and the frame's bodies where they stand.
pub fn screws(
    measure: &Value,
    cabinets: &[String],
    name: &str,
) -> Vec<(String, String, usize, [[f64; 3]; 2])> {
    let all = measure["bodies"].as_array().cloned().unwrap_or_default();
    let boxed = |b: &Value| {
        let at = |end: &str| [0, 1, 2].map(|k| b["bbox"][end][k].as_f64().unwrap_or(f64::NAN));
        [at("min"), at("max")]
    };
    let frame = format!("{name} Frame ");
    let boards: Vec<(String, [[f64; 3]; 2])> = (all.iter())
        .filter_map(|b| Some((b["name"].as_str()?.to_string(), boxed(b))))
        .filter(|(n, _)| n.starts_with(&frame))
        .collect();
    let mut holes = Vec::new();
    // a split corner cabinet's bottom is two boards
    let bottoms = cabinets
        .iter()
        .flat_map(|c| [format!("{c} Bottom"), format!("{c} Bottom return")]);
    for bottom in bottoms {
        let Some(under) = all.iter().find(|b| b["name"] == bottom.as_str()).map(boxed) else {
            continue;
        };
        for (board, b) in &boards {
            // its long way, and the middle of its thickness across it
            let k = usize::from(b[1][1] - b[0][1] > b[1][0] - b[0][0]);
            let mid = (b[0][1 - k] + b[1][1 - k]) / 2.0;
            let (lo, hi) = (under[0][k].max(b[0][k]), under[1][k].min(b[1][k]));
            let on = (under[0][2] - b[1][2]).abs() < 0.5
                && mid > under[0][1 - k] + SCREW_D
                && mid < under[1][1 - k] - SCREW_D;
            if !on || hi - lo < 100.0 {
                continue;
            }
            for (n, f) in [0.25, 0.75].into_iter().enumerate() {
                let mut hole = [[0.0; 3]; 2];
                let along = lo + (hi - lo) * f;
                for (axis, at) in [(k, along), (1 - k, mid)] {
                    hole[0][axis] = at - SCREW_D / 2.0;
                    hole[1][axis] = at + SCREW_D / 2.0;
                }
                (hole[0][2], hole[1][2]) = (under[0][2], under[1][2]);
                holes.push((bottom.clone(), board.clone(), n + 1, hole));
            }
        }
    }
    holes
}

/// A screw hole's feature name: its bottom, number and board, and where it is in the world to
/// 0.1 mm, so a frame made again with other sizes gives other names.
pub fn screw_name(screw: &(String, String, usize, [[f64; 3]; 2])) -> String {
    let (bottom, board, n, w) = screw;
    let at = |i: usize| ((w[0][i] + w[1][i]) / 2.0 * 10.0).round() / 10.0;
    format!(
        "{bottom} frame screw {n} into {board} at {}, {}",
        at(0),
        at(1)
    )
}

/// The frame screw features of `cabinet`'s bottoms in the timeline (`had`) that are not
/// `wanted` now: a run again deletes them before it cuts the new ones.
pub fn stale_screws<'a>(had: &[&'a str], wanted: &[String], cabinet: &str) -> Vec<&'a str> {
    let mine = format!("{cabinet} Bottom");
    (had.iter().copied())
        .filter(|f| {
            f.split_once(" frame screw ")
                .is_some_and(|(b, _)| b == mine || b == format!("{mine} return"))
        })
        .filter(|f| !wanted.iter().any(|w| w == f))
        .collect()
}

#[cfg(test)]
mod tests;

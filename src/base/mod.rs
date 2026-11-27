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
    let h = kit.size(&p, "frame_height", 70.0)?;
    let t = kit.size(&p, "thickness", 18.0)?;
    // mm only (no Units field), so the dialog's bounds are these
    check(
        (fittings::CASTER_MIN..=300.0).contains(&caster),
        "caster height must be 50 to 300 mm",
    )?;
    check(
        (30.0..=300.0).contains(&h),
        "frame height must be 30 to 300 mm",
    )?;
    check((3.0..=50.0).contains(&t), "thickness must be 3 to 50 mm")?;
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
    // meet along its long way, along it where they stand side by side across it (a narrow run).
    // ponytail: two cross boards that cross overlap, to be halved into each other; the cut list
    // gives both their whole length, as halving needs.
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
        n += 1;
        board(
            &format!("Frame cross {n}"),
            lo + t,
            j - t / 2.0,
            len - 2.0 * t,
            t,
        );
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

#[cfg(test)]
mod tests;

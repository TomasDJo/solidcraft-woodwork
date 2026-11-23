//! A leg-and-rail base for a workbench, table, desk or bench: square legs, aprons between them
//! under the top, and optional stretchers low down, joined with dowels or bolts. X is width, Y
//! depth (front at 0), Z height, as the cabinet; `Kit` centres it on the origin.
//!
//! The sizes are design parameters (`Frame_width`, ...) and every part and hole an expression
//! over them. The parts are solid timber: the cut list lists them bought to length (`is_timber`).

use crate::kit::{check, mm, Kit};
use crate::Commands;
use serde_json::{json, Value};

pub const DOWEL_D: f64 = 8.0;
pub const DOWEL_INTO: f64 = 20.0; // into each part: an 8 x 40 mm dowel
pub const BOLT_D: f64 = 10.0; // bed bolt and barrel nut
pub const BOLT_PAST: f64 = 80.0; // how far the bolt reaches into the rail past the leg
pub const NUT_FROM_END: f64 = 60.0; // the barrel nut's hole, from the rail's end

/// The body a frame is recognised by (`select::owner`).
pub const MARK: &str = " Leg front left";

/// Whether a body is one of a frame's parts, listed as solid timber bought to length: `Leg
/// <words>` (not a cabinet's numbered `Leg <n>`), `Apron ...` or `Stretcher ...`.
pub fn is_timber(body: &str) -> bool {
    let named = [" Leg ", " Apron ", " Stretcher "]
        .iter()
        .any(|w| body.contains(w));
    named && !crate::fittings::is_leg(body)
}

/// A rail between two legs: an apron or a stretcher.
struct Rail {
    part: String,
    along: usize,    // 0: along X, 1: along Y
    at: [String; 3], // front left bottom corner
    len: String,
    legs: [String; 2], // at its start and end
    bolted: [bool; 2], // that end bolted through the leg's outer face; else dowels
    inner: i32,        // the way into the rail from its inner face, across it
}

pub fn build(p: &Value) -> Result<(Commands, Value), String> {
    let kit = Kit::new(p)?;
    let size = |key: &str, default_mm: f64| kit.size(p, key, default_mm);
    let (w, d, h) = (
        size("width", 1200.0)?,
        size("depth", 600.0)?,
        size("height", 870.0)?,
    );
    let (l, ah, at, inset) = (
        size("leg", 70.0)?,
        size("apron_height", 100.0)?,
        size("apron_thickness", 22.0)?,
        size("inset", 10.0)?,
    );
    let sh = size("stretcher_height", 150.0)?;
    let choice = |key: &str, default: &'static str, all: &[&str]| -> Result<String, String> {
        let v = p.get(key).and_then(Value::as_str).unwrap_or(default);
        check(
            all.contains(&v),
            &format!("`{key}` must be {}", all.join(" or ")),
        )?;
        Ok(v.to_string())
    };
    let stretchers = choice("stretchers", "ends", &["none", "ends", "all"])?;
    let six = choice("legs", "4", &["4", "6"])? == "6";
    let bolts = choice("joint", "dowels", &["dowels", "bolts"])? == "bolts";
    check(
        p["place"] != "above",
        "`place`: a frame stands on the floor, not on top of a cabinet",
    )?;
    check((20.0..=300.0).contains(&l), "`leg` must be 20 to 300 mm")?;
    // more than 100 mm between each pair of legs side by side
    let legs_across = if six { 3.0 } else { 2.0 };
    let room = legs_across * l + (legs_across - 1.0) * 100.0;
    check(
        w > room && w <= 5000.0,
        &format!(
            "`width` must be more than {room} mm (100 mm between the legs) and at most 5000 mm"
        ),
    )?;
    check(
        d > 2.0 * l + 100.0 && d <= 3000.0,
        &format!(
            "`depth` must be more than {} mm (100 mm between the legs) and at most 3000 mm",
            2.0 * l + 100.0
        ),
    )?;
    check(
        (200.0..=2000.0).contains(&h),
        "`height` must be 200 to 2000 mm",
    )?;
    check(
        ah >= 40.0 && ah < h,
        "`apron_height` must be at least 40 mm and less than the height",
    )?;
    check(at >= 12.0, "`apron_thickness` must be at least 12 mm")?;
    check(inset >= 0.0, "`inset` cannot be negative")?;
    check(
        inset + at <= l,
        "`inset` and `apron_thickness` together must not be more than the leg",
    )?;
    // Dowels from two sides of a leg must not meet: at a corner the two rails' dowels cross
    // (same heights), at a middle leg the split rails' dowels come from both sides.
    let dowel = DOWEL_INTO + DOWEL_D / 2.0;
    let corner = inset + at / 2.0 + dowel;
    check(
        bolts || l > corner,
        &format!("`leg` must be more than {corner} mm for the dowels at a corner (inset, half the apron thickness and a dowel): a thicker leg, less inset or bolts"),
    )?;
    check(
        !six || l >= 2.0 * dowel + 2.0,
        &format!("`leg` must be at least {} mm with 6 legs: the dowels from both sides of a middle leg would meet", 2.0 * dowel + 2.0),
    )?;
    // Bolts from both ends of a rail must not meet inside it.
    let bolted = 2.0 * BOLT_PAST;
    check(
        !bolts || d - 2.0 * l >= bolted,
        &format!("`depth` must leave {bolted} mm between the legs for the bolts from both ends of a rail"),
    )?;
    check(
        !bolts || six || w - 2.0 * l >= bolted,
        &format!("`width` must leave {bolted} mm between the legs for the bolts from both ends of a rail"),
    )?;
    if stretchers != "none" {
        check(
            sh > 0.0 && sh + ah + 50.0 <= h - ah,
            "`stretcher_height` must be more than 0 and leave 50 mm under the aprons",
        )?;
    }

    let parameter = |key: &str, v: f64, comment: &str| kit.parameter(key, v, comment);
    let ew = parameter("width", w, "outside width, legs included");
    let ed = parameter("depth", d, "outside depth, legs included");
    let eh = parameter("height", h, "height to the top of the legs and aprons");
    let el = parameter("leg", l, "legs' square section");
    let eah = parameter("apron_height", ah, "aprons' and stretchers' height");
    let eat = parameter("apron_thickness", at, "aprons' and stretchers' thickness");
    let ein = parameter("inset", inset, "aprons set back from the legs' outer faces");
    let esh = (stretchers != "none").then(|| {
        parameter(
            "stretcher_height",
            sh,
            "floor to the stretchers' bottom face",
        )
    });
    kit.centre(&ew, &ed);
    kit.seat("L", false, &mm(0.0), &ed);
    kit.seat("R", false, &ew, &ed);

    let zero = mm(0.0);
    let name = &kit.name;
    // Legs: the corners, and with six a middle pair.
    let (right, back, mid) = (
        format!("{ew} - {el}"),
        format!("{ed} - {el}"),
        format!("{ew}/2 - {el}/2"),
    );
    let mut legs = vec![
        ("Leg front left", &zero),
        ("Leg front right", &right),
        ("Leg back left", &zero),
        ("Leg back right", &right),
    ];
    if six {
        legs.extend([("Leg middle front", &mid), ("Leg middle back", &mid)]);
    }
    for (part, x) in &legs {
        let y = if part.contains("back") { &back } else { &zero };
        kit.panel(part, [x, y, &zero], [&el, &el, &eh], None);
    }
    let leg = |part: &str| format!("{name} Leg {part}");

    // The rails at one height `z`: `kind` is Apron or Stretcher; `ends` only the end ones.
    let rails = |kind: &str, z: &str, ends: bool| -> Vec<Rail> {
        let rail = |part: &str, along, at: [String; 3], len: String, legs, bolted, inner| Rail {
            part: format!("{kind} {part}"),
            along,
            at,
            len,
            legs,
            bolted,
            inner,
        };
        let z = z.to_string();
        let span = format!("{ed} - 2*{el}");
        let (xl, xr) = (ein.clone(), format!("{ew} - {ein} - {eat}"));
        let mut all = vec![
            rail(
                "L",
                1,
                [xl, el.clone(), z.clone()],
                span.clone(),
                [leg("front left"), leg("back left")],
                [true, true],
                -1,
            ),
            rail(
                "R",
                1,
                [xr, el.clone(), z.clone()],
                span.clone(),
                [leg("front right"), leg("back right")],
                [true, true],
                1,
            ),
        ];
        if six {
            all.push(rail(
                "middle",
                1,
                [format!("{ew}/2 - {eat}/2"), el.clone(), z.clone()],
                span,
                [leg("middle front"), leg("middle back")],
                [true, true],
                -1,
            ));
        }
        if ends {
            return all;
        }
        let (yf, yb) = (ein.clone(), format!("{ed} - {ein} - {eat}"));
        for (side, y, inner, at) in [("front", yf, -1, "front"), ("back", yb, 1, "back")] {
            let corner = |s: &str| leg(&format!("{at} {s}"));
            if six {
                let half = format!("{ew}/2 - {el}*3/2");
                let middle = leg(&format!("middle {at}"));
                all.push(rail(
                    &format!("{side} L"),
                    0,
                    [el.clone(), y.clone(), z.clone()],
                    half.clone(),
                    [corner("left"), middle.clone()],
                    [true, false],
                    inner,
                ));
                all.push(rail(
                    &format!("{side} R"),
                    0,
                    [format!("{ew}/2 + {el}/2"), y, z.clone()],
                    half,
                    [middle, corner("right")],
                    [false, true],
                    inner,
                ));
            } else {
                all.push(rail(
                    side,
                    0,
                    [el.clone(), y, z.clone()],
                    format!("{ew} - 2*{el}"),
                    [corner("left"), corner("right")],
                    [true, true],
                    inner,
                ));
            }
        }
        all
    };
    // ponytail: a stretcher has the apron's section; give it its own when a bench needs it.
    let mut all = rails("Apron", &format!("{eh} - {eah}"), false);
    if let Some(esh) = &esh {
        all.extend(rails("Stretcher", esh, stretchers == "ends"));
    }
    for r in &all {
        let mut size = [eat.as_str(); 3];
        size[r.along] = &r.len;
        size[2] = &eah;
        kit.panel(&r.part, [&r.at[0], &r.at[1], &r.at[2]], size, None);
    }

    // The joints, each a cylinder cut out of the leg and the rail at once.
    let mut n = [0; 3]; // dowels, bolts, barrel nuts
    let mut cut = |what: usize,
                   base: [String; 3],
                   axis: [i32; 3],
                   dia: f64,
                   depth: String,
                   targets: Vec<String>| {
        n[what] += 1;
        let called = ["dowel", "bolt", "barrel nut"][what];
        kit.push((
            "solid.cylinder",
            json!({
                "base": kit.at(&base[0], &base[1], &base[2]), "axis": axis,
                "diameter": mm(dia), "height": depth, "operation": "cut",
                "targets": targets, "name": format!("{name} {called} {}", n[what]),
            }),
        ));
    };
    for r in &all {
        let a = r.along;
        let across = 1 - a;
        let body = format!("{name} {}", r.part);
        let middle = format!("{} + {eat}/2", r.at[across]);
        for end in 0..2 {
            // the joint's plane, and the way into the rail from it
            let (plane, into) = match end {
                0 => (r.at[a].clone(), 1),
                _ => (format!("{} + {}", r.at[a], r.len), -1),
            };
            let mut dir = [0; 3];
            dir[a] = into;
            let targets = vec![r.legs[end].clone(), body.clone()];
            let point = |along: String, cross: String, z: String| {
                let mut p = [String::new(), String::new(), z];
                p[a] = along;
                p[across] = cross;
                p
            };
            if bolts && r.bolted[end] {
                // through the leg from its outer face; X and Y rails at two heights so the
                // bolts in a corner leg do not meet
                let z = match a {
                    0 => format!("{} + {eah}*2/3", r.at[2]),
                    _ => format!("{} + {eah}/3", r.at[2]),
                };
                let outer = format!("{plane} - ({into})*{el}");
                let depth = format!("{el} + {}", mm(BOLT_PAST));
                cut(
                    1,
                    point(outer, middle.clone(), z.clone()),
                    dir,
                    BOLT_D,
                    depth,
                    targets,
                );
                // the barrel nut from the rail's inner face, across it to past the bolt
                let along = format!("{plane} + ({into})*{}", mm(NUT_FROM_END));
                let face = match r.inner {
                    -1 => format!("{} + {eat}", r.at[across]),
                    _ => r.at[across].clone(),
                };
                let mut way = [0; 3];
                way[across] = r.inner;
                let depth = format!("{eat}/2 + {}", mm(BOLT_D / 2.0));
                cut(
                    2,
                    point(along, face, z),
                    way,
                    BOLT_D,
                    depth,
                    vec![body.clone()],
                );
            } else {
                for k in [1, 3] {
                    let z = format!("{} + {eah}*{k}/4", r.at[2]);
                    let base = format!("{plane} - ({into})*{}", mm(DOWEL_INTO));
                    let depth = mm(2.0 * DOWEL_INTO);
                    cut(
                        0,
                        point(base, middle.clone(), z),
                        dir,
                        DOWEL_D,
                        depth,
                        targets.clone(),
                    );
                }
            }
        }
    }
    Ok(kit.finish(Vec::new(), [w, h, d]))
}

#[cfg(test)]
mod tests;

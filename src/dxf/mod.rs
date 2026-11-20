//! A sheet's cutting layout as an ASCII DXF (R12), for a CNC router or a CAD program: the
//! sheet, each part's outline and name, and its holes on a layer per diameter and depth. In mm,
//! x along the sheet's length and y across it, as the cutting diagrams.

use serde_json::{json, Value};

type V = [f64; 3];

fn v(value: &Value) -> Option<V> {
    Some([value[0].as_f64()?, value[1].as_f64()?, value[2].as_f64()?])
}

fn dot(a: V, b: V) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: V, b: V) -> V {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// A number as DXF and layer names show it: to 0.01 (0.1 in names), no trailing `.0`.
fn num(x: f64, step: f64) -> String {
    format!("{}", (x / step).round() * step + 0.0)
}

/// The layer of a hole: `DRILL_5_D12`, `DRILL_35_D13_BACK`, `DRILL_8_THRU`, `DRILL_4P5_D10P9`
/// (R12 layer names take no `.`). `depth` is none for a hole right through.
pub fn layer(diameter: f64, depth: Option<f64>, back: bool) -> String {
    let n = |x: f64| num(x, 0.1).replace('.', "P");
    let d = n(diameter);
    let deep = depth.map_or("THRU".into(), |depth| format!("D{}", n(depth)));
    let back = if back { "_BACK" } else { "" };
    format!("DRILL_{d}_{deep}{back}")
}

/// A part's holes on the sheet, from `model.faces` of its board (world coordinates, any
/// placement) and `part`, its place in a layout (`x, y, w, h, rotated`). Each hole is
/// `[x, y, diameter, depth or null for through, back]`; back is a hole that opens on the face
/// underneath as drawn. Also the holes into an edge, which a sheet drawing cannot show.
/// The board's own axes come from its flat faces: the normal of the biggest is its thickness,
/// that of an edge face and the cross of the two its length and width, matched to the part's
/// size on the sheet. A hole's depth comes from where its round face's centroid lies, half way
/// down it (the face areas are of the tessellation, a few percent short).
// ponytail: positions are from the board's middle, so at Cut sizes a hole is off by up to
// half the banding (1 mm) toward a banded edge. Grooves and pockets are not drawn.
pub fn holes(faces: &Value, part: &Value) -> (Vec<Value>, usize) {
    let all = faces["faces"].as_array().map_or(&[][..], |f| f);
    let flat: Vec<(V, V, f64)> = (all.iter())
        .filter_map(|f| {
            Some((
                v(&f["plane_normal"])?,
                v(&f["centroid"])?,
                f["area"].as_f64()?,
            ))
        })
        .collect();
    let Some(&(t, ..)) = flat.iter().max_by(|a, b| a.2.total_cmp(&b.2)) else {
        return (Vec::new(), 0);
    };
    let edge = flat.iter().filter(|f| dot(f.0, t).abs() < 0.01);
    let Some(&(e2, ..)) = edge.max_by(|a, b| a.2.total_cmp(&b.2)) else {
        return (Vec::new(), 0);
    };
    let e3 = cross(t, e2);
    // the board's extent along an axis, from its faces square to it
    let span = |axis: V| {
        let on = flat.iter().filter(|f| dot(f.0, axis).abs() > 0.999);
        let at: Vec<f64> = on.map(|f| dot(f.1, axis)).collect();
        let lo = at.iter().copied().fold(f64::INFINITY, f64::min);
        (lo, at.iter().copied().fold(f64::NEG_INFINITY, f64::max))
    };
    let (s2, s3, st) = (span(e2), span(e3), span(t));
    let size = |key: &str| part[key].as_f64().unwrap_or(0.0);
    let rotated = part["rotated"] == true;
    let (length, width) = match rotated {
        true => (size("h"), size("w")),
        false => (size("w"), size("h")),
    };
    let (n2, n3) = (s2.1 - s2.0, s3.1 - s3.0);
    // ponytail: a square or nearly square part (length and width within the banding) can
    // be matched the wrong way round, its holes turned a quarter against the grain the
    // nesting kept; picking the axis as `cutlist` does would need its axis on the layout.
    let ((u, su), (w, sw)) = match (n2 - length).abs() + (n3 - width).abs()
        <= (n3 - length).abs() + (n2 - width).abs()
    {
        true => ((e2, s2), (e3, s3)),
        false => ((e3, s3), (e2, s2)),
    };
    // Drawn with the length along x and the width along y, the board is seen from this side
    // of its thickness: the + side when length, width and thickness turn right-handed.
    let seen_plus = dot(cross(u, w), t) > 0.0;
    let (mid_u, mid_w) = ((su.0 + su.1) / 2.0, (sw.0 + sw.1) / 2.0);
    let mut seen = std::collections::BTreeSet::new();
    let (mut found, mut into_edge) = (Vec::new(), 0);
    for face in all {
        let c = &face["cylinder"];
        let (Some(origin), Some(axis), Some(r), Some(centroid)) = (
            v(&c["origin"]),
            v(&c["axis"]),
            c["radius"].as_f64(),
            v(&face["centroid"]),
        ) else {
            continue;
        };
        // a round face cut in two by the kernel is one hole
        let key = (
            origin.map(|x| (x * 10.0).round() as i64),
            (r * 100.0).round() as i64,
        );
        if !seen.insert(key) {
            continue;
        }
        if dot(axis, t).abs() < 0.999 {
            into_edge += 1;
            continue;
        }
        let (a, b) = (dot(origin, u) - mid_u, dot(origin, w) - mid_w);
        let ct = dot(centroid, t);
        let (above, below) = (st.1 - ct, ct - st.0);
        let depth = ((above - below).abs() > 0.05).then(|| 2.0 * above.min(below));
        let back = depth.is_some() && (above < below) != seen_plus;
        found.push((a, b, 2.0 * r, depth, back));
    }
    // Seen from the face with the most holes: a board can be laid either face up.
    if found.iter().filter(|h| h.4).count() * 2 > found.len() {
        for h in &mut found {
            h.1 = -h.1;
            h.4 = !h.4 && h.3.is_some();
        }
    }
    let (x, y) = (size("x") + size("w") / 2.0, size("y") + size("h") / 2.0);
    let drawn = found.into_iter().map(|(a, b, d, depth, back)| {
        // turned a quarter: the length runs up the sheet
        let (dx, dy) = if rotated { (-b, a) } else { (a, b) };
        json!([x + dx, y + dy, d, depth, back])
    });
    (drawn.collect(), into_edge)
}

fn pair(out: &mut String, code: u32, value: &str) {
    out.push_str(&format!(
        "{code}
{value}
"
    ));
}

/// A closed rectangle: R12 has no LWPOLYLINE, so a POLYLINE of four VERTEXes.
fn rect(out: &mut String, layer: &str, x: f64, y: f64, w: f64, h: f64) {
    pair(out, 0, "POLYLINE");
    pair(out, 8, layer);
    pair(out, 66, "1");
    pair(out, 70, "1"); // closed
    for (px, py) in [(x, y), (x + w, y), (x + w, y + h), (x, y + h)] {
        pair(out, 0, "VERTEX");
        pair(out, 8, layer);
        pair(out, 10, &num(px, 0.01));
        pair(out, 20, &num(py, 0.01));
        pair(out, 30, "0");
    }
    pair(out, 0, "SEQEND");
    pair(out, 8, layer);
}

/// One sheet's layout (`size, parts` as `cutlist` lays it out, a part with its `holes` from
/// `holes` when they were read) as DXF R12 text. `trim` is the edge trim in mm, drawn inside
/// the sheet's outline.
pub fn render(layout: &Value, trim: f64) -> String {
    let at = |value: &Value, i: usize| value[i].as_f64().unwrap_or(0.0);
    let (length, width) = (at(&layout["size"], 0), at(&layout["size"], 1));
    let mut layers = vec!["SHEET".to_string()];
    let mut e = String::new();
    rect(&mut e, "SHEET", 0.0, 0.0, length, width);
    if trim > 0.0 {
        rect(
            &mut e,
            "SHEET",
            trim,
            trim,
            length - 2.0 * trim,
            width - 2.0 * trim,
        );
    }
    for part in layout["parts"].as_array().into_iter().flatten() {
        let size = |key: &str| part[key].as_f64().unwrap_or(0.0);
        let (x, y, w, h) = (size("x"), size("y"), size("w"), size("h"));
        rect(&mut e, "OUTLINE", x, y, w, h);
        // The name along the part's longer side, as big as fits.
        let name = part["part"].as_str().unwrap_or("");
        let chars = name.chars().count().max(1) as f64;
        let height = (0.9 * w.max(h) / chars).min(0.3 * w.min(h)).min(40.0);
        let (cx, cy) = (num(x + w / 2.0, 0.01), num(y + h / 2.0, 0.01));
        pair(&mut e, 0, "TEXT");
        pair(&mut e, 8, "TEXT");
        for (code, value) in [(10, &cx), (20, &cy), (11, &cx), (21, &cy)] {
            pair(&mut e, code, value);
        }
        pair(&mut e, 40, &num(height, 0.01));
        pair(&mut e, 1, name);
        if h > w {
            pair(&mut e, 50, "90");
        }
        pair(&mut e, 72, "1"); // centred
        pair(&mut e, 73, "2"); // on the middle
        for hole in part["holes"].as_array().into_iter().flatten() {
            let layer = layer(at(hole, 2), hole[3].as_f64(), hole[4] == true);
            pair(&mut e, 0, "CIRCLE");
            pair(&mut e, 8, &layer);
            pair(&mut e, 10, &num(at(hole, 0), 0.01));
            pair(&mut e, 20, &num(at(hole, 1), 0.01));
            pair(&mut e, 30, "0");
            pair(&mut e, 40, &num(at(hole, 2) / 2.0, 0.01));
            if !layers.contains(&layer) {
                layers.push(layer);
            }
        }
    }
    if layout["parts"].as_array().is_some_and(|p| !p.is_empty()) {
        layers.splice(1..1, ["OUTLINE".to_string(), "TEXT".to_string()]);
    }
    let mut out = String::new();
    for (code, value) in [
        (0, "SECTION"),
        (2, "HEADER"),
        (9, "$ACADVER"),
        (1, "AC1009"),
    ] {
        pair(&mut out, code, value);
    }
    for (code, value) in [(9, "$INSUNITS"), (70, "4"), (0, "ENDSEC")] {
        pair(&mut out, code, value); // 4: millimetres, for readers newer than R12
    }
    for (code, value) in [(0, "SECTION"), (2, "TABLES"), (0, "TABLE"), (2, "LTYPE")] {
        pair(&mut out, code, value);
    }
    for (code, value) in [(70, "1"), (0, "LTYPE"), (2, "CONTINUOUS"), (70, "0")] {
        pair(&mut out, code, value);
    }
    for (code, value) in [(3, "Solid line"), (72, "65"), (73, "0"), (40, "0")] {
        pair(&mut out, code, value);
    }
    for (code, value) in [(0, "ENDTAB"), (0, "TABLE"), (2, "LAYER")] {
        pair(&mut out, code, value);
    }
    pair(&mut out, 70, &layers.len().to_string());
    for name in &layers {
        // colours: grey sheet, white outlines, green text, red holes, blue from the back
        let colour = match name.as_str() {
            "SHEET" => "8",
            "OUTLINE" => "7",
            "TEXT" => "3",
            _ if name.ends_with("_BACK") => "5",
            _ => "1",
        };
        for (code, value) in [
            (0, "LAYER"),
            (2, name),
            (70, "0"),
            (62, colour),
            (6, "CONTINUOUS"),
        ] {
            pair(&mut out, code, value);
        }
    }
    for (code, value) in [
        (0, "ENDTAB"),
        (0, "ENDSEC"),
        (0, "SECTION"),
        (2, "ENTITIES"),
    ] {
        pair(&mut out, code, value);
    }
    out.push_str(&e);
    pair(&mut out, 0, "ENDSEC");
    pair(&mut out, 0, "EOF");
    out
}

/// The files a DXF cut list is saved as, one per layout: `<stem>-<material>-<thickness>-<n>.dxf`,
/// n counting the sheets of each material and thickness from 1. The material keeps only what a
/// file name may have.
pub fn files(stem: &str, layouts: &[Value]) -> Vec<String> {
    let mut count = std::collections::BTreeMap::new();
    (layouts.iter())
        .map(|layout| {
            let material: String = (layout["material"].as_str().unwrap_or("").chars())
                .filter(|c| c.is_alphanumeric() || " -_".contains(*c))
                .collect();
            let thickness = num(layout["thickness"].as_f64().unwrap_or(0.0), 0.1);
            let n = count
                .entry((material.clone(), thickness.clone()))
                .or_insert(0);
            *n += 1;
            format!("{stem}-{material}-{thickness}-{n}.dxf")
        })
        .collect()
}

#[cfg(test)]
mod tests;

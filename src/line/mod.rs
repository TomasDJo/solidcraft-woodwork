//! Place: Along a line (#147). A picked sketch line, model edge or wall face (its bottom edge) as
//! a level segment along X or Y, which side the cabinet's front faces, and where on it the
//! cabinet stands. Plain geometry: `lib.rs` reads the line (`sketch.inspect`, `model.edges`,
//! `model.faces`) and moves the cabinet.

use crate::kit::check;
use serde_json::Value;

/// A level line along X or Y, in world mm, and the way a cabinet's front faces from it.
#[derive(Debug, PartialEq)]
pub struct Line {
    pub a: [f64; 3],
    pub b: [f64; 3],
    /// Unit, level, square to the line: away from the wall (a body's face), or for a sketch
    /// line toward the middle of the sketch's other points.
    pub front: [f64; 2],
    /// A sketch line's driving length dimension: its parameter's name.
    pub dimension: Option<String>,
}

fn point(v: &Value) -> Option<[f64; 3]> {
    let p = v.as_array()?;
    Some([
        p.first()?.as_f64()?,
        p.get(1)?.as_f64()?,
        p.get(2)?.as_f64()?,
    ])
}

/// Level and along X or Y (quarter turns only: `select`, Place and the worktop read world
/// boxes), with `front` made square to it: `toward` picks its side, else -Y or -X.
fn level(a: [f64; 3], b: [f64; 3], toward: Option<[f64; 2]>) -> Result<Line, String> {
    let d = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    check(len > 1.0, "the line is shorter than 1 mm")?;
    check(
        d[2].abs() < 1e-6 * len.max(1.0),
        "the line must be level (a floor line or a wall's bottom edge)",
    )?;
    let along_x = d[1].abs() < 1e-6 * len;
    if !along_x && d[0].abs() >= 1e-6 * len {
        let degrees = d[1].atan2(d[0]).to_degrees();
        return Err(format!(
            "the line must run along X or Y (quarter turns only); this one is at {degrees:.1} degrees"
        ));
    }
    let mut front = if along_x { [0.0, -1.0] } else { [-1.0, 0.0] };
    if let Some(t) = toward {
        if t[0] * front[0] + t[1] * front[1] < 0.0 {
            front = [-front[0], -front[1]];
        }
    }
    Ok(Line {
        a,
        b,
        front,
        dimension: None,
    })
}

/// A sketch line from `sketch.inspect`'s reply, by curve id ("l1"). It faces the middle of the
/// sketch's other points (the room it is drawn in); its driving length dimension, if it has one,
/// is `dimension`.
pub fn from_sketch(sketch: &Value, curve: &str) -> Result<Line, String> {
    let (a, b, dimension, others) = sketch_line(sketch, curve)?;
    let toward = (!others.is_empty()).then(|| {
        let n = others.len() as f64;
        let c = others
            .iter()
            .fold([0.0, 0.0], |s, p| [s[0] + p[0] / n, s[1] + p[1] / n]);
        [c[0] - (a[0] + b[0]) / 2.0, c[1] - (a[1] + b[1]) / 2.0]
    });
    let mut found = level(a, b, toward.filter(|t| t[0].abs() + t[1].abs() > 1e-6))?;
    found.dimension = dimension;
    Ok(found)
}

/// Height from line: a sketch line's length (any way it runs) and its driving dimension.
pub fn sketch_length(sketch: &Value, curve: &str) -> Result<(f64, Option<String>), String> {
    let (a, b, dimension, _) = sketch_line(sketch, curve)?;
    Ok((span(a, b), dimension))
}

/// A model edge's length, by index (`model.edges` of its body).
pub fn edge_length(edges: &Value, index: u64) -> Result<f64, String> {
    let all = edges["edges"].as_array().cloned().unwrap_or_default();
    let edge = all.iter().find(|e| e["index"].as_u64() == Some(index));
    let edge = edge.ok_or("the picked edge is not on its body any more")?;
    edge["length"]
        .as_f64()
        .ok_or("the picked edge has no length".into())
}

fn span(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2) + (b[2] - a[2]).powi(2)).sqrt()
}

/// A sketch line's ends in world mm, its driving length dimension's parameter, and the world
/// places of the sketch's other points.
#[allow(clippy::type_complexity)]
fn sketch_line(
    sketch: &Value,
    curve: &str,
) -> Result<([f64; 3], [f64; 3], Option<String>, Vec<[f64; 3]>), String> {
    let curves = sketch["curves"].as_array().cloned().unwrap_or_default();
    let line = curves.iter().find(|c| c["id"] == curve);
    let line = line.ok_or(format!("the sketch has no curve `{curve}`"))?;
    check(line["type"] == "line", "pick a straight line")?;
    let points = sketch["points"].as_array().cloned().unwrap_or_default();
    let at = |id: &Value| {
        let p = points.iter().find(|p| p["id"] == *id)?;
        point(&p["world"])
    };
    let (a, b) = at(&line["start"])
        .zip(at(&line["end"]))
        .ok_or("the line's ends are not in the sketch")?;
    let others: Vec<[f64; 3]> = (points.iter())
        .filter(|p| p["id"] != "origin" && p["id"] != line["start"] && p["id"] != line["end"])
        .filter_map(|p| point(&p["world"]))
        .collect();
    let index = curves.iter().position(|c| c["id"] == curve);
    let constraints = sketch["constraints"].as_array().into_iter().flatten();
    let dimension = (constraints)
        .filter(|c| c["type"] == "length" && c["driven"] != true)
        .find(|c| c["l"].as_u64().map(|l| l as usize) == index)
        .and_then(|c| c["param"].as_str())
        .map(String::from);
    Ok((a, b, dimension, others))
}

/// The front of a body's flat face: its normal, when it is upright.
fn upright(faces: &Value, name: &str) -> Option<[f64; 2]> {
    let all = faces["faces"].as_array()?;
    let n = all.iter().find(|f| f["name"] == name)?["plane_normal"]
        .as_array()?
        .clone();
    let n = [
        n.first()?.as_f64()?,
        n.get(1)?.as_f64()?,
        n.get(2)?.as_f64()?,
    ];
    (n[2].abs() < 1e-6).then_some([n[0], n[1]])
}

/// A model edge by index (`model.edges` and `model.faces` of its body): the front is away from
/// the upright face it bounds.
pub fn from_edge(edges: &Value, faces: &Value, index: u64) -> Result<Line, String> {
    let all = edges["edges"].as_array().cloned().unwrap_or_default();
    let edge = all.iter().find(|e| e["index"].as_u64() == Some(index));
    let edge = edge.ok_or("the picked edge is not on its body any more")?;
    let ends = point(&edge["start"]).zip(point(&edge["end"]));
    let (a, b) = ends.ok_or("pick a straight edge")?;
    let length = edge["length"].as_f64().unwrap_or(0.0);
    let span = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2) + (b[2] - a[2]).powi(2)).sqrt();
    check(
        (span - length).abs() < 1e-3 * length.max(1.0),
        "pick a straight edge",
    )?;
    let names = edge["name"].as_str().unwrap_or_default();
    let toward = names.split('|').find_map(|n| upright(faces, n));
    level(a, b, toward)
}

/// A wall face by index: its bottom edge, the front away from the face.
pub fn from_face(edges: &Value, faces: &Value, index: u64) -> Result<Line, String> {
    let all = faces["faces"].as_array().cloned().unwrap_or_default();
    let face = all.iter().find(|f| f["index"].as_u64() == Some(index));
    let name = face
        .and_then(|f| f["name"].as_str())
        .ok_or("the picked face is not on its body any more")?;
    check(
        upright(faces, name).is_some(),
        "pick an upright flat face, such as a wall's side",
    )?;
    let bounds = |e: &&Value| {
        e["name"]
            .as_str()
            .is_some_and(|n| n.split('|').any(|f| f == name))
    };
    let z = |e: &Value| {
        point(&e["start"])
            .zip(point(&e["end"]))
            .map(|(a, b)| (a[2] + b[2]) / 2.0)
    };
    let edges_of = edges["edges"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(bounds);
    let bottom = edges_of.min_by(|p, q| {
        z(p).unwrap_or(f64::MAX)
            .total_cmp(&z(q).unwrap_or(f64::MAX))
    });
    let bottom = bottom.ok_or("the face has no edges")?;
    from_edge(edges, faces, bottom["index"].as_u64().unwrap_or(u64::MAX))
}

/// The front, flipped if asked.
pub fn facing(line: &Line, flip: bool) -> [f64; 2] {
    if flip {
        [-line.front[0], -line.front[1]]
    } else {
        line.front
    }
}

/// Along the cabinet's width (its right, seen from the front): the front turned a quarter.
fn right(front: [f64; 2]) -> [f64; 2] {
    [-front[1], front[0]]
}

/// The line's ends along `right`, the left one first.
fn ends(line: &Line, right: [f64; 2]) -> (f64, f64) {
    let along = |p: [f64; 3]| p[0] * right[0] + p[1] * right[1];
    let (s, e) = (along(line.a), along(line.b));
    (s.min(e), s.max(e))
}

pub fn length(line: &Line) -> f64 {
    ((line.b[0] - line.a[0]).powi(2) + (line.b[1] - line.a[1]).powi(2)).sqrt()
}

/// Width from line: the line less the offset (at the middle, less it at each end).
pub fn width(line: &Line, start: &str, offset: f64) -> f64 {
    match start {
        "middle" => length(line) - 2.0 * offset.abs(),
        _ => length(line) - offset,
    }
}

/// Where the cabinet's origin goes and its turn about Z in degrees: its back on the line, its
/// front facing `front`, its left side `offset` from the left end (or its right side from the
/// right end; or its middle `offset` right of the line's middle). `width` and `depth` are its
/// box's, in mm.
pub fn place(
    line: &Line,
    front: [f64; 2],
    start: &str,
    offset: f64,
    width: f64,
    depth: f64,
) -> ([f64; 3], i32) {
    let r = right(front);
    let (left, right_end) = ends(line, r);
    let along = match start {
        "right" => right_end - offset - width / 2.0,
        "middle" => (left + right_end) / 2.0 + offset,
        _ => left + offset + width / 2.0,
    };
    // the line's own place across it, from either end
    let across = line.a[0] * front[0] + line.a[1] * front[1] + depth / 2.0;
    let x = along * r[0] + across * front[0];
    let y = along * r[1] + across * front[1];
    let degrees = match (front[0].round() as i32, front[1].round() as i32) {
        (1, 0) => 90,
        (0, 1) => 180,
        (-1, 0) => -90,
        _ => 0,
    };
    ([x, y, line.a[2]], degrees)
}

/// `place` as `occurrence.move`'s translation, with the box's `size` (width and depth as
/// expressions, `Kit`'s `centre`) in it: the side at `start` stays where it is and the back on
/// the line when the cabinet's parameters change (at the middle it grows to both sides).
pub fn translate(
    line: &Line,
    front: [f64; 2],
    start: &str,
    offset: f64,
    size: [&str; 2],
) -> ([String; 3], i32) {
    let (base, degrees) = place(line, front, start, offset, 0.0, 0.0);
    let r = right(front);
    let grows = match start {
        "right" => -1.0,
        "middle" => 0.0,
        _ => 1.0,
    };
    let mm = |v: f64| format!("{} mm", (v * 1e6).round() / 1e6);
    let axis = |i: usize| {
        let mut e = mm(base[i]);
        for (c, s) in [(grows * r[i], size[0]), (front[i], size[1])] {
            if c.abs() > 0.5 {
                e += &format!(" {} ({s})/2", if c > 0.0 { "+" } else { "-" });
            }
        }
        e
    };
    ([axis(0), axis(1), mm(base[2])], degrees)
}

/// A free parameter name for a sketch line's dimension: `<Sketch>_<curve>`, then `_2`, ...
pub fn dimension_name(sketch: &str, curve: &str, design: &Value) -> String {
    let stem = format!("{}_{curve}", crate::kit::prefix(sketch));
    let all = design["params"].as_array().cloned().unwrap_or_default();
    let taken = |n: &str| all.iter().any(|p| p["name"] == n);
    (1..)
        .map(|i| {
            if i == 1 {
                stem.clone()
            } else {
                format!("{stem}_{i}")
            }
        })
        .find(|n| !taken(n))
        .unwrap_or(stem)
}

/// Whether a parameter name is one the app gave (`d1`, `d12`): unnamed.
pub fn unnamed(param: &str) -> bool {
    param
        .strip_prefix('d')
        .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
}

#[cfg(test)]
mod tests;

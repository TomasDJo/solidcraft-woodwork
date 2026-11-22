//! Worktop over the selected cabinets: one piece per straight run, joined in the corners
//! (butt or mitre), with optional dog holes and a cut-out in the first piece.
//!
//! Numeric, not parametric: the cabinets are measured where they stand (`inspect.measure`'s
//! world boxes) and each piece is a box, or a sketch outline extruded for a mitred piece, at
//! world coordinates in a component of its own that stays on the origin. `lib.rs` makes that
//! component, and replaces it when Worktop runs again with the same name.

use crate::kit::{check, Kit};
use crate::select::{self, owner, Plan};
use crate::Commands;
use serde_json::{json, Value};

const TOLERANCE: f64 = 0.5; // mm
const DOG: f64 = 20.0; // dog hole diameter, mm
const GRID: f64 = 96.0; // between dog holes, mm: the MFT pattern
const EDGE: f64 = 60.0; // least from a dog hole to the piece's edge, mm

// ponytail: the tool's shelf is a fixed 18 mm board on 18 x 40 battens, not fields.
const BOARD: f64 = 18.0; // the tool shelf's thickness, mm
const BATTEN: [f64; 2] = [18.0, 40.0]; // a batten under it: thick against the side, high

type V = [f64; 2];

fn dot(a: V, b: V) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}

/// How far a plan box reaches along a direction: lowest and highest.
fn span(b: Plan, d: V) -> (f64, f64) {
    let (a, c) = (dot([b[0], b[1]], d), dot([b[2], b[3]], d));
    (a.min(c), a.max(c))
}

/// The part of `w` within `depth` of its edge in the direction `back`.
fn strip(w: Plan, back: V, depth: f64) -> Plan {
    let mut s = w;
    match back {
        [x, _] if x > 0.5 => s[0] = w[2] - depth,
        [x, _] if x < -0.5 => s[2] = w[0] + depth,
        [_, y] if y > 0.5 => s[1] = w[3] - depth,
        _ => s[3] = w[1] + depth,
    }
    s
}

/// Rounded to a millionth of a mm, so 600 does not come out as 599.9999999999999.
fn round(v: f64) -> f64 {
    (v * 1e6).round() / 1e6
}

/// A length in mm for the app, rounded.
fn mm(v: f64) -> String {
    crate::kit::mm(round(v))
}

/// A plan box of a cabinet, or of one leg of a corner cabinet, and the way its front faces.
struct Foot {
    cabinet: String,
    plan: Plan,
    front: V,
}

/// A cabinet's feet and the height of its top. A box cabinet has one, its sides and back seen
/// from above. A corner cabinet (its two sides stand square to each other) has a leg ending in
/// each side, the whole length of the cabinet that way and as deep, from its wall, as that
/// side is long, facing the way the other side faces out.
fn feet(measure: &Value, name: &str) -> Result<(Vec<Foot>, f64), String> {
    let c = select::named(measure, name)?;
    let mut whole = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for body in measure["bodies"].as_array().into_iter().flatten() {
        let part = body["name"].as_str().unwrap_or_default();
        let Some(part) = part.strip_prefix(name).and_then(|p| p.strip_prefix(' ')) else {
            continue;
        };
        if !["Side L", "Side R", "Back", "Back return"].contains(&part) {
            continue;
        }
        let at = |end: &str, i: usize| body["bbox"][end][i].as_f64().unwrap_or(f64::NAN);
        // A side turned off a quarter turn no longer fills its box.
        let size: f64 = (0..3).map(|i| at("max", i) - at("min", i)).product();
        check(
            !part.starts_with("Side") || body["volume_mm3"].as_f64().is_none_or(|v| v > 0.8 * size),
            &format!("`{name}` is turned off a quarter turn: a worktop needs cabinets square to the axes"),
        )?;
        whole = [
            whole[0].min(at("min", 0)),
            whole[1].min(at("min", 1)),
            whole[2].max(at("max", 0)),
            whole[3].max(at("max", 1)),
        ];
    }
    let out = |side: &str| {
        let d = c.toward(side).0;
        [d[0], d[1]]
    };
    let (l, r) = (out("left"), out("right"));
    let feet = match dot(l, r) == 0.0 {
        true => vec![(c.sides[0], r), (c.sides[1], l)]
            .into_iter()
            .map(|(side, front)| {
                let (lo, hi) = span(side, front);
                let back = front.map(|v| -v);
                (strip(whole, back, hi - lo), front)
            })
            .collect(),
        false => vec![(whole, [r[1], -r[0]])],
    };
    let feet = feet.into_iter();
    let feet = feet.map(|(plan, front)| Foot {
        cabinet: name.to_string(),
        plan,
        front,
    });
    Ok((feet.collect(), c.max[2]))
}

/// A straight run of cabinets facing the same way: the way its right is seen from the front,
/// where its wall is and how deep it is to the worktop's front edge (along -front), and where
/// its cabinets start and end along it.
struct Run {
    front: V,
    right: V,
    wall: f64,
    depth: f64,
    ends: [f64; 2],
    first: String,
}

impl Run {
    fn back(&self) -> V {
        self.front.map(|v| -v)
    }

    /// Where `other`'s wall and its front edge are, along this run.
    fn across(&self, other: &Run) -> (f64, f64) {
        let s = dot(other.back(), self.right);
        (other.wall * s, (other.wall - other.depth) * s)
    }
}

/// One piece of the worktop: its run's directions, wall and depth, and where it ends along the
/// run at the back and at the front edge, left and right. A mitred end is not square.
#[derive(Clone)]
pub struct Piece {
    right: V,
    back: V,
    wall: f64,
    depth: f64,
    left: (f64, f64),
    right_end: (f64, f64),
}

impl Piece {
    fn square(&self) -> bool {
        (self.left.0 - self.left.1).abs() < 1e-6
            && (self.right_end.0 - self.right_end.1).abs() < 1e-6
    }

    /// Its left end, at the back or the front edge, whichever is further left.
    fn start(&self) -> f64 {
        self.left.0.min(self.left.1)
    }

    /// Its length along the run, what it is bought at.
    pub fn length(&self) -> f64 {
        self.right_end.0.max(self.right_end.1) - self.start()
    }

    /// A point in the world from one on the piece: `x` from its left end, `y` from its front
    /// edge towards the wall.
    fn world(&self, x: f64, y: f64) -> V {
        let (u, w) = (self.start() + x, self.wall - self.depth + y);
        [
            round(u * self.right[0] + w * self.back[0]),
            round(u * self.right[1] + w * self.back[1]),
        ]
    }

    /// Its outline in the world: back left, back right, front right, front left.
    fn outline(&self) -> Vec<V> {
        let s = self.start();
        [
            (self.left.0, self.depth),
            (self.right_end.0, self.depth),
            (self.right_end.1, 0.0),
            (self.left.1, 0.0),
        ]
        .map(|(u, y)| self.world(u - s, y))
        .to_vec()
    }

    /// Whether a point on the piece (as `world` takes it) is on it, its edge included.
    fn holds(&self, x: f64, y: f64) -> bool {
        let t = y / self.depth; // 0 at the front edge, 1 at the wall
        let u = self.start() + x;
        let lo = self.left.1 + (self.left.0 - self.left.1) * t;
        let hi = self.right_end.1 + (self.right_end.0 - self.right_end.1) * t;
        (-1e-6..=self.depth + 1e-6).contains(&y) && u >= lo - 1e-6 && u <= hi + 1e-6
    }

    /// Whether a rectangle on the piece, x0, y0, x1, y1, lies on it.
    fn fits(&self, r: [f64; 4]) -> bool {
        [(r[0], r[1]), (r[2], r[1]), (r[2], r[3]), (r[0], r[3])]
            .iter()
            .all(|&(x, y)| self.holds(x, y))
    }

    /// The square part, where the piece is whole from the front edge to the wall: x0, x1.
    fn whole(&self) -> (f64, f64) {
        let s = self.start();
        (
            self.left.0.max(self.left.1) - s,
            self.right_end.0.min(self.right_end.1) - s,
        )
    }

    /// A rectangle on the piece in the world: corner and size along X and Y.
    fn world_box(&self, r: [f64; 4]) -> (V, V) {
        let (a, b) = (self.world(r[0], r[1]), self.world(r[2], r[3]));
        let lo = [a[0].min(b[0]), a[1].min(b[1])];
        (
            lo,
            [round(a[0].max(b[0]) - lo[0]), round(a[1].max(b[1]) - lo[1])],
        )
    }
}

/// The selected cabinets' worktop pieces, longest first, and the height of its underside.
pub fn plan(
    measure: &Value,
    cabinets: &[String],
    joint: &str,
    overhangs: [f64; 3],
) -> Result<(Vec<Piece>, f64), String> {
    let [front_overhang, left_overhang, right_overhang] = overhangs;
    check(
        !cabinets.is_empty(),
        "select the cabinets for the worktop first",
    )?;
    let mut all: Vec<Foot> = vec![];
    let mut top: Option<(f64, &str)> = None;
    for name in cabinets {
        let (feet, height) = feet(measure, name)?;
        if let Some((first, one)) = top {
            check(
                (height - first).abs() <= TOLERANCE,
                &format!("`{name}` is not as high as `{one}`: the worktop needs every cabinet's top at the same height"),
            )?;
        }
        top = top.or(Some((height, name)));
        all.extend(feet);
    }
    // The feet by the way they face: a run each.
    let mut runs: Vec<Run> = vec![];
    let mut fronts: Vec<V> = vec![];
    for foot in &all {
        if !fronts.contains(&foot.front) {
            fronts.push(foot.front);
        }
    }
    for front in fronts {
        let feet: Vec<&Foot> = all.iter().filter(|f| f.front == front).collect();
        let (right, back) = ([-front[1], front[0]], front.map(|v| -v));
        let wall = (feet.iter().map(|f| span(f.plan, back).1)).fold(f64::NEG_INFINITY, f64::max);
        let carcass = (feet.iter().map(|f| span(f.plan, back).0)).fold(f64::INFINITY, f64::min);
        let behind = feet.iter().find(|f| span(f.plan, back).1 == wall);
        for foot in &feet {
            check(
                wall - span(foot.plan, back).1 <= TOLERANCE,
                &format!(
                    "`{}` and `{}` face the same way but do not stand against the same wall: make one worktop per run, with different names",
                    behind.map(|f| f.cabinet.as_str()).unwrap_or_default(),
                    foot.cabinet
                ),
            )?;
        }
        let lo = (feet.iter().map(|f| span(f.plan, right).0)).fold(f64::INFINITY, f64::min);
        let hi = (feet.iter().map(|f| span(f.plan, right).1)).fold(f64::NEG_INFINITY, f64::max);
        runs.push(Run {
            front,
            right,
            wall,
            depth: wall - carcass + front_overhang,
            ends: [lo, hi],
            first: feet[0].cabinet.clone(),
        });
    }
    // Two runs facing each other are two worktops, unless a run across joins them (a U).
    for a in &runs {
        for b in &runs {
            let joined = runs.iter().any(|c| dot(c.front, a.front) == 0.0);
            check(
                dot(a.front, b.front) != -1.0 || joined,
                &format!(
                    "`{}` and `{}` face opposite ways: make one worktop per run, with different names",
                    a.first, b.first
                ),
            )?;
        }
    }
    // Longest first: it passes through a butt joint, and is piece 1.
    runs.sort_by(|a, b| (b.ends[1] - b.ends[0]).total_cmp(&(a.ends[1] - a.ends[0])));
    // Each end: where it stops at the wall and at the front edge. A free end overhangs.
    let mut cuts: Vec<[(f64, f64); 2]> = (runs.iter())
        .map(|r| {
            let (l, r) = (r.ends[0] - left_overhang, r.ends[1] + right_overhang);
            [(l, l), (r, r)]
        })
        .collect();
    let mut turned = vec![[false; 2]; runs.len()];
    // Which end of run `a` turns into run `b`: the one nearest `b`'s wall, which must be at or
    // beyond it, with `b` facing back along `a`.
    let meet = |a: &Run, b: &Run| -> Result<usize, String> {
        let (wall, _) = a.across(b);
        let end = match (wall - a.ends[0]).abs() <= (wall - a.ends[1]).abs() {
            true => 0,
            false => 1,
        };
        let facing = dot(b.front, a.right);
        let ok = match end {
            0 => facing > 0.0 && wall <= a.ends[0] + TOLERANCE,
            _ => facing < 0.0 && wall >= a.ends[1] - TOLERANCE,
        };
        check(
            ok,
            &format!(
                "the runs of `{}` and `{}` do not meet in a corner: make one worktop per run, with different names",
                a.first, b.first
            ),
        )?;
        Ok(end)
    };
    for i in 0..runs.len() {
        for j in i + 1..runs.len() {
            if dot(runs[i].front, runs[j].front) != 0.0 {
                continue;
            }
            let (a, b) = (&runs[i], &runs[j]);
            let (ea, eb) = (meet(a, b)?, meet(b, a)?);
            check(
                !turned[i][ea] && !turned[j][eb],
                &format!("`{}` turns two corners at one end", a.first),
            )?;
            (turned[i][ea], turned[j][eb]) = (true, true);
            let ((wall_b, front_b), (wall_a, front_a)) = (a.across(b), b.across(a));
            match joint {
                // Both to the walls, cut from the inner corner to the outer one.
                "mitre" => {
                    cuts[i][ea] = (wall_b, front_b);
                    cuts[j][eb] = (wall_a, front_a);
                }
                // The longer through to the other's wall, the other against its front edge.
                _ => {
                    cuts[i][ea] = (wall_b, wall_b);
                    cuts[j][eb] = (front_a, front_a);
                }
            }
        }
    }
    let pieces: Vec<Piece> = (runs.iter().zip(cuts))
        .map(|(run, [left, right_end])| Piece {
            right: run.right,
            back: run.back(),
            wall: run.wall,
            depth: run.depth,
            left,
            right_end,
        })
        .collect();
    for (piece, run) in pieces.iter().zip(&runs) {
        check(
            piece.right_end.0 - piece.left.0 > TOLERANCE
                && piece.right_end.1 - piece.left.1 > TOLERANCE,
            &format!("the worktop over `{}` has no length left", run.first),
        )?;
    }
    Ok((pieces, top.map(|t| t.0).unwrap_or_default()))
}

/// The cabinet under a tool's opening on `piece` (a rectangle on it, as `fits` takes it): the
/// one of `cabinets` whose two sides, each square to the run, stand either side of the opening
/// with its middle within their depth. Its name, the tool shelf's rectangle on the piece
/// (between the sides' inner faces, from the back's front face to the sides' front edge),
/// and its floor: the top of its `Bottom`, or its lowest point without one.
fn under(
    measure: &Value,
    cabinets: &[String],
    piece: &Piece,
    opening: [f64; 4],
) -> Result<(String, [f64; 4], f64), String> {
    let (s, y0) = (piece.start(), piece.wall - piece.depth);
    let (u0, u1) = (s + opening[0], s + opening[2]);
    let middle = y0 + (opening[1] + opening[3]) / 2.0;
    let bbox = |name: String| {
        let all = measure["bodies"].as_array().into_iter().flatten();
        all.into_iter()
            .find(|b| b["name"] == name.as_str())
            .cloned()
    };
    for name in cabinets {
        let c = select::named(measure, name)?;
        let [l, r] = c
            .sides
            .map(|side| (span(side, piece.right), span(side, piece.back)));
        let square = |(a, w): ((f64, f64), (f64, f64))| a.1 - a.0 < w.1 - w.0;
        let (left, right) = match l.0 .0 <= r.0 .0 {
            true => (l, r),
            false => (r, l),
        };
        let front = left.1 .0.max(right.1 .0);
        let back = left.1 .1.min(right.1 .1);
        let fits = square(l) && square(r) && left.0 .1 <= u0 + 1e-6 && u1 <= right.0 .0 + 1e-6;
        if !fits || !(front..=back).contains(&middle) {
            continue;
        }
        // A grooved back stands between the sides, an overlaid one behind them.
        let back = match c.back == Plan::default() {
            true => back,
            false => span(c.back, piece.back).0.min(back),
        };
        let floor = match bbox(format!("{name} Bottom")) {
            Some(b) => b["bbox"]["max"][2].as_f64().unwrap_or(c.min[2]),
            None => c.min[2],
        };
        let shelf = [left.0 .1 - s, front - y0, right.0 .0 - s, back - y0];
        check(
            opening[3] <= shelf[3] + 1e-6,
            "the tool's opening reaches over the cabinet's back",
        )?;
        return Ok((name.clone(), shelf, floor));
    }
    Err("the tool's opening must lie between the sides of one cabinet".into())
}

/// The boards of `cabinet` that `space` (a world box: min, max) passes through and that may
/// be cut (`cuttable`: the top and a sink base's rails, which get the tool's opening too). Any
/// other body of the cabinet in it (a shelf, a divider, a door) is refused; its sides and back
/// may touch it.
fn crossing(
    measure: &Value,
    cabinet: &str,
    space: [[f64; 3]; 2],
    cuttable: &[&str],
) -> Result<Vec<String>, String> {
    let mut cut = vec![];
    for body in measure["bodies"].as_array().into_iter().flatten() {
        let name = body["name"].as_str().unwrap_or_default();
        if select::owner(measure, name) != Some(cabinet) {
            continue;
        }
        let at = |end: &str, i: usize| body["bbox"][end][i].as_f64().unwrap_or(f64::NAN);
        let inside = (0..3).all(|i| {
            at("min", i) < space[1][i] - TOLERANCE && at("max", i) > space[0][i] + TOLERANCE
        });
        let part = &name[cabinet.len() + 1..];
        if !inside || ["Side L", "Side R", "Back"].contains(&part) {
            continue;
        }
        check(
            cuttable.contains(&part),
            &format!("`{name}` is in the tool's way: make the cabinet without it"),
        )?;
        cut.push(name.to_string());
    }
    Ok(cut)
}

/// A world box (min, max) in the cabinet's own frame, where `placed` (`edit::placement`'s
/// matrix: three rows of a turn and a shift) puts it: corner and size, as `solid.box` takes
/// them. The cabinet's parent is taken to stand on the origin, as the worktop's does.
pub fn local(world: [[f64; 3]; 2], placed: &Value) -> ([f64; 3], [f64; 3]) {
    let m = |r: usize, c: usize| placed[r][c].as_f64().unwrap_or(0.0);
    // the turn is square, so its inverse is its transpose
    let back = |p: [f64; 3]| {
        let d = [p[0] - m(0, 3), p[1] - m(1, 3), p[2] - m(2, 3)];
        [0, 1, 2].map(|c| round((0..3).map(|r| m(r, c) * d[r]).sum()))
    };
    let (a, b) = (back(world[0]), back(world[1]));
    let lo = [0, 1, 2].map(|i| a[i].min(b[i]));
    (lo, [0, 1, 2].map(|i| round(a[i].max(b[i]) - lo[i])))
}

/// How many dog holes fit between `lo` and `hi`, their centres at least `margin` in, on the
/// grid and centred, and where the first one's centre is. None when not even one fits.
fn grid(lo: f64, hi: f64, margin: f64) -> Option<(usize, f64)> {
    let room = hi - lo - 2.0 * margin;
    if room < -1e-9 {
        return None;
    }
    let n = (room / GRID + 1e-9).floor() as usize + 1;
    Some((n, (lo + hi) / 2.0 - (n - 1) as f64 * GRID / 2.0))
}

/// The body name of piece `n` (from 0) of `count`: the name alone for one piece, else numbered.
fn body(name: &str, n: usize, count: usize) -> String {
    match count {
        1 => name.to_string(),
        _ => format!("{name} {}", n + 1),
    }
}

/// Whether `body` is a piece of the worktop `name`: the name, or the name and a number; or
/// one of its tool's parts.
pub fn piece_of(body: &str, name: &str) -> bool {
    let rest = body.strip_prefix(name);
    rest.is_some_and(|r| {
        r.is_empty()
            || [" Tool shelf", " Tool batten L", " Tool batten R"].contains(&r)
            || r.strip_prefix(' ')
                .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
    })
}

/// The components of which `body` may be a piece, the longest name first: of "Worktop" and
/// "Worktop 2", the body "Worktop 2" is the second's own before it is the first's piece 2.
pub fn owners<'a>(body: &str, components: &[&'a str]) -> Vec<&'a str> {
    let mut found: Vec<&str> = (components.iter().copied())
        .filter(|c| piece_of(body, c))
        .collect();
    found.sort_by_key(|c| std::cmp::Reverse(c.len()));
    found
}

/// The worktop's commands, made in its own component that stays on the origin, and its
/// summary. `cabinets` are the cabinets under it, `measure` is `inspect.measure`'s reply.
pub fn build(p: &Value, cabinets: &[String], measure: &Value) -> Result<(Commands, Value), String> {
    let mut p = p.clone();
    if p.get("name").is_none() {
        p["name"] = json!("Worktop");
    }
    let kit = Kit::new(&p)?;
    let name = kit.name.clone();
    // Body names are the data model: a cabinet must not take a piece for one of its parts.
    if let Some(cabinet) = owner(measure, &format!("{name} 1")) {
        return Err(format!(
            "name: `{name}` would read as a part of the cabinet `{cabinet}`; choose another"
        ));
    }
    let size = |key: &str, default: f64| -> Result<f64, String> {
        let v = kit.size(&p, key, default)?;
        check(v >= 0.0, &format!("`{key}` must not be negative"))?;
        Ok(v)
    };
    let choice = |key: &str, default: &'static str, all: &[&str]| -> Result<String, String> {
        let v = p[key].as_str().unwrap_or(default);
        check(
            all.contains(&v),
            &format!("{key} must be one of {}", all.join(", ")),
        )?;
        Ok(v.to_string())
    };
    let thickness = size("thickness", 38.0)?;
    check(thickness > 0.0, "thickness must be more than 0")?;
    let joint = choice("joint", "butt", &["butt", "mitre"])?;
    let overhangs = [
        size("front_overhang", 40.0)?,
        size("left_overhang", 0.0)?,
        size("right_overhang", 0.0)?,
    ];
    let (pieces, top) = plan(measure, cabinets, &joint, overhangs)?;
    let count = pieces.len();
    let names: Vec<String> = (0..count).map(|n| body(&name, n, count)).collect();
    for (piece, body) in pieces.iter().zip(&names) {
        if piece.square() {
            let (at, size) = piece.world_box([0.0, 0.0, piece.length(), piece.depth]);
            kit.push((
                "solid.box",
                json!({
                    "length": mm(size[0]), "width": mm(size[1]), "height": mm(thickness),
                    "corner": [at[0], at[1], top], "operation": "new",
                    "name": body, "body_name": body,
                }),
            ));
            continue;
        }
        let sketch = format!("{body} outline");
        kit.push((
            "sketch.create",
            json!({"plane": {"origin": [0, 0, top], "x_dir": [1, 0, 0], "y_dir": [0, 1, 0]}, "name": sketch}),
        ));
        kit.push((
            "sketch.line",
            json!({"points": piece.outline(), "closed": true}),
        ));
        kit.push(("sketch.finish", json!({})));
        kit.push((
            "solid.extrude",
            json!({
                "sketch": sketch, "distance": mm(thickness), "operation": "new",
                "name": body, "body_name": body,
            }),
        ));
    }
    // Dog holes and the cut-out go into the first piece, the longest.
    let (first, target) = (&pieces[0], &names[0]);
    let up = |v: V| json!([v[0], v[1], 0]);
    let mut holes = 0;
    let dogs = choice("dog_holes", "none", &["none", "all", "zone"])?;
    if dogs != "none" {
        let (x, y) = first.whole();
        let (area, margin) = match dogs.as_str() {
            "all" => ([x, 0.0, y, first.depth], EDGE + DOG / 2.0),
            _ => {
                let (zx, zy) = (size("zone_x", 100.0)?, size("zone_y", 100.0)?);
                let zone = [
                    zx,
                    zy,
                    zx + size("zone_length", 800.0)?,
                    zy + size("zone_width", 400.0)?,
                ];
                check(
                    first.fits(zone),
                    &format!("the dog hole zone does not fit on `{target}`"),
                )?;
                (zone, DOG / 2.0)
            }
        };
        let too_small = format!("there is no room for a dog hole on `{target}` ({EDGE} mm from its edges, in the zone if given)");
        let (nx, x0) = grid(area[0], area[2], margin).ok_or(&too_small)?;
        let (ny, y0) = grid(area[1], area[3], margin).ok_or(&too_small)?;
        let hole = format!("{name} dog hole");
        let at = first.world(x0, y0);
        kit.push((
            "solid.cylinder",
            json!({
                "diameter": mm(DOG), "height": mm(thickness), "base": [at[0], at[1], top],
                "axis": "Z", "operation": "cut", "targets": [target], "name": hole,
            }),
        ));
        // One cylinder and a pattern of it, so a big top stays within the command limit.
        let rows: Vec<(V, usize)> = [(first.right, nx), (first.back, ny)]
            .into_iter()
            .filter(|row| row.1 > 1)
            .collect();
        if !rows.is_empty() {
            let mut args = json!({
                "features": [hole], "name": format!("{name} dog holes"),
                "dir1": up(rows[0].0), "count1": rows[0].1, "spacing1": mm(GRID),
            });
            if let Some(&(dir, n)) = rows.get(1) {
                args["dir2"] = up(dir);
                args["count2"] = json!(n);
                args["spacing2"] = json!(mm(GRID));
            }
            kit.push(("solid.pattern.rectangular", args));
        }
        holes = nx * ny;
    }
    let cutout = choice("cutout", "none", &["none", "plain", "rebated"])?;
    if cutout != "none" {
        let (l, w) = (size("cutout_length", 500.0)?, size("cutout_width", 400.0)?);
        check(
            l > 0.0 && w > 0.0,
            "the cut-out's length and width must be more than 0",
        )?;
        let (cx, cy) = (size("cutout_x", 600.0)?, size("cutout_y", 300.0)?);
        let opening = [cx - l / 2.0, cy - w / 2.0, cx + l / 2.0, cy + w / 2.0];
        let mut outer = opening;
        let cut = |key: &str, r: [f64; 4], z: f64, height: f64| {
            let (at, size) = first.world_box(r);
            kit.push((
                "solid.box",
                json!({
                    "length": mm(size[0]), "width": mm(size[1]), "height": mm(height),
                    "corner": [at[0], at[1], z], "operation": "cut", "targets": [target],
                    "name": format!("{name} {key}"),
                }),
            ));
        };
        if cutout == "rebated" {
            let (rw, rd) = (size("rebate_width", 10.0)?, size("rebate_depth", 10.0)?);
            check(
                rw > 0.0 && rd > 0.0 && rd < thickness,
                "the rebate must be wider than 0 and less deep than the worktop is thick",
            )?;
            outer = [
                opening[0] - rw,
                opening[1] - rw,
                opening[2] + rw,
                opening[3] + rw,
            ];
            cut("rebate", outer, top + thickness - rd, rd);
        }
        check(
            first.fits(outer),
            &format!("the cut-out does not fit on `{target}`"),
        )?;
        cut("cut-out", opening, top, thickness);
    }
    let mut parts = names.clone();
    let mut tool = Value::Null;
    if choice("tool", "none", &["none", "benchtop"])? == "benchtop" {
        let (w, d, h) = (
            size("tool_width", 640.0)?,
            size("tool_depth", 540.0)?,
            size("tool_height", 300.0)?,
        );
        check(
            w > 0.0 && d > 0.0 && h > 0.0,
            "the tool's width, depth and height must be more than 0",
        )?;
        let (tx, ty, gap) = (
            size("tool_x", 600.0)?,
            size("tool_y", 300.0)?,
            size("tool_clearance", 3.0)?,
        );
        let (w, d) = (w / 2.0 + gap, d / 2.0 + gap);
        let opening = [tx - w, ty - d, tx + w, ty + d];
        check(
            first.fits(opening),
            &format!("the tool's opening does not fit on `{target}`"),
        )?;
        let (cabinet, shelf, floor) = under(measure, cabinets, first, opening)?;
        let surface = top + thickness;
        let shelf_top = surface - h;
        check(
            shelf_top <= top + 1e-6,
            "tool height: the tool is shorter than the worktop is thick, so the shelf would be above the worktop's underside",
        )?;
        check(
            shelf_top - BOARD - BATTEN[1] >= floor - 1e-6,
            &format!("tool height: the tool is taller than `{cabinet}` is deep inside, so the shelf would be below the cabinet's bottom"),
        )?;
        // The tool's space, from its shelf up to the worktop: the cabinet's top and rails get
        // the opening too, anything else of the cabinet there is in the way.
        let (at, size) = first.world_box(opening);
        let space = [
            [at[0], at[1], shelf_top],
            [at[0] + size[0], at[1] + size[1], top],
        ];
        let cut = crossing(
            measure,
            &cabinet,
            space,
            &["Top", "Rail front", "Rail back"],
        )?;
        // and nothing where the shelf and its battens go
        let (at, size) = first.world_box(shelf);
        let under = [
            [at[0], at[1], shelf_top - BOARD - BATTEN[1]],
            [at[0] + size[0], at[1] + size[1], shelf_top],
        ];
        crossing(measure, &cabinet, under, &[])?;
        let piece = |key: &str, r: [f64; 4], z: f64, height: f64, operation: &str| {
            let (at, size) = first.world_box(r);
            let body = format!("{name} {key}");
            let mut args = json!({
                "length": mm(size[0]), "width": mm(size[1]), "height": mm(height),
                "corner": [at[0], at[1], round(z)], "operation": operation, "name": body,
            });
            match operation {
                "cut" => args["targets"] = json!([target]),
                _ => args["body_name"] = json!(body),
            }
            kit.push(("solid.box", args));
            body
        };
        piece("tool opening", opening, top, thickness, "cut");
        let bodies = vec![
            piece("Tool shelf", shelf, shelf_top - BOARD, BOARD, "new"),
            piece(
                "Tool batten L",
                [shelf[0], shelf[1], shelf[0] + BATTEN[0], shelf[3]],
                shelf_top - BOARD - BATTEN[1],
                BATTEN[1],
                "new",
            ),
            piece(
                "Tool batten R",
                [shelf[2] - BATTEN[0], shelf[1], shelf[2], shelf[3]],
                shelf_top - BOARD - BATTEN[1],
                BATTEN[1],
                "new",
            ),
        ];
        parts.extend(bodies.iter().cloned());
        tool = json!({
            "cabinet": cabinet, "shelf_z": round(shelf_top), "bodies": bodies,
            "cut": {"targets": cut, "min": space[0], "max": space[1]},
        });
    }
    let (commands, _) = kit.finish(vec![], [0.0; 3]);
    let mut summary = json!({
        "name": name,
        "bodies": parts,
        "pieces": (pieces.iter().zip(&names))
            .map(|(piece, body)| json!({"name": body, "length": round(piece.length()), "depth": round(piece.depth)}))
            .collect::<Vec<_>>(),
        "thickness": thickness,
        "joint": joint,
        "dog_holes": holes,
        "cabinets": cabinets,
    });
    if !tool.is_null() {
        summary["tool"] = tool;
    }
    Ok((commands, summary))
}

#[cfg(test)]
mod tests;

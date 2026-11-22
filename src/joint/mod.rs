//! Joinery chosen when a cabinet is built: one module the builders share. A [`Joint`] is
//! where the end of one board meets the face of another (a side and the bottom); the
//! cabinet's [`Joinery`] knows the method for it and its sizes, and returns the cuts.
//!
//! Every size is a design parameter of the cabinet (`<Prefix>_joint_spacing`, `_dowel_d`,
//! ...) and every cut an expression over its parameters, so the joints follow a resize: a row
//! of holes is one cut and a `solid.pattern.rectangular` whose count follows the joint's
//! length. The builders cut them while the cabinet is still on its component's origin, where a
//! pattern keeps its copies (see CLAUDE.md).
//!
//! The cut list counts the fasteners from a parameter per item (`<Prefix>_dowels`, ...): the
//! sum of the rows' counts, so it follows a resize too.

use crate::kit::{check, mm, Kit};
use crate::Commands;
use serde_json::{json, Value};
use std::cell::RefCell;

/// The methods, as the dialog's `joints` field gives them. `none` leaves the boards butted.
pub const METHODS: [&str; 3] = ["none", "dowels", "confirmat"];

/// The sizes a method can take: dialog field, which is also the parameter `<Prefix>_<key>`,
/// and its default in mm.
const SIZES: [(&str, f64); 4] = [
    ("joint_spacing", 150.0),
    ("joint_edge", 50.0),
    ("dowel_d", 8.0),
    ("dowel_length", 35.0),
];

/// The parameters that count the fasteners for the cut list, `<Prefix>_<key>`: 0 once an edit
/// took them away. `edit::merge` passes the ones a cabinet has to `Kit`.
pub const COUNTS: [&str; 2] = ["dowels", "confirmats"];

const CONFIRMAT: [f64; 4] = [7.0, 5.0, 50.0, 40.0]; // clearance, pilot, length, thickest board

/// Where the end of one board meets the face of another. Positions are counted from the
/// cabinet's front left bottom corner, as a panel's (`Kit::at` shifts them).
pub struct Joint {
    /// The board whose face the other's end meets (a side), and that other board (the bottom).
    pub face: String,
    pub end: String,
    /// On the joint's plane, in the middle of the end board's thickness, where the row starts.
    pub at: [String; 3],
    /// From the joint's plane into the end board.
    pub into: [i32; 3],
    /// The way the row runs, and its length.
    pub along: [i32; 3],
    pub run: String,
    /// One fastener in the middle, not a row: a sink base's rails, 100 mm deep.
    pub single: bool,
}

/// A cabinet's joinery: a method for the carcass (sides to bottom and top) and one for the
/// boards inside (dividers and the rail), with their sizes as parameters.
pub struct Joinery<'a> {
    kit: &'a Kit,
    t: String,
    methods: [String; 2],
    sizes: Vec<(&'static str, String)>,
    counts: RefCell<Vec<(&'static str, String)>>,
    cuts: RefCell<Commands>,
}

/// Which of a cabinet's two methods a joint takes: the carcass's, or the inside's.
pub const CARCASS: usize = 0;
pub const INNER: usize = 1;

impl<'a> Joinery<'a> {
    /// Reads `joints` (the carcass) and `joints_inner` (dividers and the rail, when `inner`
    /// has any) and makes the parameters their sizes need. A cabinet that remembers no method
    /// (made before joinery was chosen at build) has none. `t` is the boards' thickness.
    pub fn new(kit: &'a Kit, p: &Value, t: (f64, &str), inner: bool) -> Result<Self, String> {
        let method = |key: &str| -> Result<String, String> {
            let v = p.get(key).and_then(Value::as_str).unwrap_or("none");
            check(
                METHODS.contains(&v),
                &format!("`{key}` must be {}", METHODS.join(", ")),
            )?;
            Ok(v.to_string())
        };
        let carcass = method("joints")?;
        let inside = match inner {
            true => method("joints_inner")?,
            false => "none".into(),
        };
        let uses = |m: &str| carcass == m || inside == m;
        let mut sizes = Vec::new();
        for (key, default) in SIZES {
            let needed = match key {
                "joint_spacing" | "joint_edge" => uses("dowels") || uses("confirmat"),
                _ => uses("dowels"),
            };
            if !needed {
                continue;
            }
            let v = kit.size(p, key, default)?;
            let (low, high) = match key {
                "joint_spacing" => (20.0, 2000.0),
                "joint_edge" => (5.0, 500.0),
                "dowel_d" => (3.0, 20.0),
                _ => (10.0, 150.0),
            };
            check(
                (low..=high).contains(&v),
                &format!("`{key}` must be {low} to {high} mm"),
            )?;
            sizes.push((key, v));
        }
        let size = |key: &str| sizes.iter().find(|s| s.0 == key).map(|s| s.1);
        if let (Some(d), Some(l)) = (size("dowel_d"), size("dowel_length")) {
            check(
                d <= t.0 - 4.0 && l > 2.0 * d,
                "a dowel must be at least 4 mm thinner than the boards and more than twice as long as it is thick",
            )?;
        }
        check(
            !uses("confirmat") || t.0 < CONFIRMAT[3],
            "boards of 40 mm or more are too thick for 50 mm confirmat screws",
        )?;
        let comment = |key: &str| match key {
            "joint_spacing" => "most between two joint fasteners",
            "joint_edge" => "first joint fastener from the board's edge",
            "dowel_d" => "dowel diameter",
            _ => "dowel length",
        };
        let sizes = (sizes.into_iter())
            .map(|(key, v)| (key, kit.parameter(key, v, comment(key))))
            .collect();
        Ok(Joinery {
            kit,
            t: t.1.to_string(),
            methods: [carcass, inside],
            sizes,
            counts: RefCell::default(),
            cuts: RefCell::default(),
        })
    }

    /// The method for the carcass (`CARCASS`) or the boards inside (`INNER`).
    pub fn method(&self, scope: usize) -> &str {
        &self.methods[scope]
    }

    fn size(&self, key: &str) -> &str {
        let found = self.sizes.iter().find(|s| s.0 == key);
        found.map(|s| s.1.as_str()).unwrap_or_default()
    }

    /// Checks that a row of `run_mm` has room for its fasteners: `what` names it.
    pub fn fits(&self, scope: usize, run_mm: f64, p: &Value, what: &str) -> Result<(), String> {
        if !matches!(self.method(scope), "dowels" | "confirmat") {
            return Ok(());
        }
        let edge = self.kit.size(p, "joint_edge", 50.0)?;
        check(
            run_mm > 2.0 * edge + 10.0,
            &format!("`joint_edge` leaves no room for the joints along {what}"),
        )
    }

    /// The cuts of one joint, by the method of `scope`; run once the boards are there.
    pub fn joint(&self, scope: usize, j: &Joint) {
        let method = self.method(scope).to_string();
        let (spacing, edge) = (self.size("joint_spacing"), self.size("joint_edge"));
        let count = format!("max(2, ceil(({} - 2*{edge}) / {spacing}) + 1)", j.run);
        let pitch = format!("({} - 2*{edge}) / ({count} - 1)", j.run);
        let start = match j.single {
            true => shift(&j.at, j.along, &format!("{} / 2", j.run)),
            false => shift(&j.at, j.along, edge),
        };
        let name = format!("{} joint {} {}", self.kit.name, j.face, j.end);
        let (face, end) = (self.board(&j.face), self.board(&j.end));
        let t = &self.t;
        let mut features = Vec::new();
        let mut cut = |what: &str, base: [String; 3], axis: [i32; 3], d: String, h: String| {
            let feature = match what {
                "" => format!("{name} {method}"),
                _ => format!("{name} {method} {what}"),
            };
            let targets = match what {
                "" if method == "dowels" => vec![face.clone(), end.clone()],
                "" => vec![face.clone()],
                _ => vec![end.clone()],
            };
            self.cylinder(&feature, base, axis, &d, &h, targets);
            features.push(feature);
        };
        let item = match method.as_str() {
            "dowels" => {
                // Blind in the face, 5 mm short of coming through; the rest in the end, with
                // 1 mm to spare at each end.
                let (d, l) = (self.size("dowel_d"), self.size("dowel_length"));
                let into_face = format!("min({t} - 5 mm, ({l} + 2 mm) / 2)");
                let base = shift(&start, neg(j.into), &into_face);
                cut("", base, j.into, d.into(), format!("{l} + 2 mm"));
                "dowels"
            }
            "confirmat" => {
                // Through the face board from its outside, then the pilot in the end.
                let [clear, pilot, length, _] = CONFIRMAT;
                let base = shift(&start, neg(j.into), t);
                cut("", base, j.into, mm(clear), t.clone());
                let depth = format!("{} - {t} + 2 mm", mm(length));
                cut("pilot", start.clone(), j.into, mm(pilot), depth);
                "confirmats"
            }
            _ => return,
        };
        if j.single {
            self.counts.borrow_mut().push((item, "1".into()));
            return;
        }
        self.cuts.borrow_mut().push((
            "solid.pattern.rectangular",
            json!({
                "features": features, "name": format!("{name} {method} row"),
                "dir1": j.along, "count1": count, "spacing1": pitch,
            }),
        ));
        self.counts.borrow_mut().push((item, count));
    }

    fn board(&self, part: &str) -> String {
        format!("{} {part}", self.kit.name)
    }

    fn cylinder(
        &self,
        name: &str,
        base: [String; 3],
        axis: [i32; 3],
        d: &str,
        h: &str,
        targets: Vec<String>,
    ) {
        let base = self.kit.at(&base[0], &base[1], &base[2]);
        self.cuts.borrow_mut().push((
            "solid.cylinder",
            json!({
                "base": base, "axis": axis, "diameter": d, "height": h,
                "operation": "cut", "targets": targets, "name": name,
            }),
        ));
    }

    /// The parameters that count the fasteners (made, changed, or 0 when no longer used),
    /// pushed to `Kit`, and the cuts, to run after every board is there.
    pub fn finish(&self) -> Commands {
        let counts = self.counts.borrow();
        let kit = self.kit;
        for item in COUNTS {
            let rows: Vec<&str> = (counts.iter())
                .filter(|c| c.0 == item)
                .map(|c| c.1.as_str())
                .collect();
            let id = format!("{}_{item}", kit.prefix);
            let expression = match rows.is_empty() {
                true if kit.has(item) => "0".to_string(),
                true => continue,
                false => rows.join(" + "),
            };
            match kit.has(item) {
                true => kit.push((
                    "parameters.change",
                    json!({"name": id, "expression": expression}),
                )),
                false => kit.push((
                    "parameters.add",
                    json!({
                        "name": id, "expression": expression,
                        "comment": format!("{}: {item} to buy", kit.name),
                    }),
                )),
            }
        }
        self.cuts.take()
    }
}

/// `at` moved `by` the way `dir` points.
fn shift(at: &[String; 3], dir: [i32; 3], by: &str) -> [String; 3] {
    let mut p = at.clone();
    for i in 0..3 {
        match dir[i] {
            1 => p[i] = format!("{} + ({by})", p[i]),
            -1 => p[i] = format!("{} - ({by})", p[i]),
            _ => {}
        }
    }
    p
}

fn neg(dir: [i32; 3]) -> [i32; 3] {
    dir.map(|v| -v)
}

/// The fasteners to buy for the cabinet with the parameter prefix `prefix`, from the design's
/// parameters (`document.inspect`'s `params`): `(item, quantity)`.
pub fn hardware(prefix: &str, params: &Value) -> Vec<(String, u64)> {
    let value = |key: &str| {
        let id = format!("{prefix}_{key}");
        let all = params.as_array().into_iter().flatten();
        all.into_iter()
            .find(|p| p["name"] == id.as_str())
            .and_then(|p| p["value"].as_f64())
    };
    let size = |key: &str| value(key).map(|v| (v * 10.0).round() / 10.0).unwrap_or(0.0);
    let mut items = Vec::new();
    for key in COUNTS {
        let qty = value(key).unwrap_or(0.0).round() as u64;
        if qty == 0 {
            continue;
        }
        let item = match key {
            "dowels" => format!("Dowel, {} x {} mm", size("dowel_d"), size("dowel_length")),
            _ => "Confirmat screw, 7 x 50 mm".to_string(),
        };
        items.push((item, qty));
    }
    items
}

#[cfg(test)]
mod tests;

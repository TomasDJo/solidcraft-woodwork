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
pub const METHODS: [&str; 8] = [
    "none",
    "dowels",
    "confirmat",
    "screws",
    "dado",
    "biscuit",
    "cam",
    "pocket",
];
/// The methods with fasteners in a row along the joint, spaced by `joint_spacing`.
const SPACED: [&str; 6] = ["dowels", "confirmat", "screws", "biscuit", "cam", "pocket"];

/// The sizes a method can take: dialog field, which is also the parameter `<Prefix>_<key>`,
/// and its default in mm.
const SIZES: [(&str, f64); 7] = [
    ("joint_spacing", 250.0),
    ("joint_edge", 50.0),
    ("dowel_d", 8.0),
    ("dowel_length", 35.0),
    ("dado_depth", 6.0),
    ("cam_d", 15.0),
    ("cam_bolt", 35.0),
];

/// The parameters that count the fasteners for the cut list, `<Prefix>_<key>`: 0 once an edit
/// took them away. `edit::merge` passes the ones a cabinet has to `Kit`.
pub const COUNTS: [&str; 9] = [
    "dowels",
    "confirmats",
    "screws",
    "biscuits_0",
    "biscuits_10",
    "biscuits_20",
    "dominos",
    "cams",
    "pocket_screws",
];

// A screw from the face board's outside into the other's end: clearance, pilot, length.
const CONFIRMAT: [f64; 3] = [7.0, 5.0, 50.0];
const SCREW: [f64; 3] = [4.5, 3.0, 50.0]; // a 4 x 50 mm chipboard screw

/// The `biscuit` field's slots: the slot's length along the joint, depth into each board and
/// thickness (a rectangle for the biscuit's half moon), the count's parameter and the item.
const SLOTS: [(&str, [f64; 3], &str, &str); 4] = [
    ("0", [53.0, 9.0, 4.0], "biscuits_0", "Biscuit, #0"),
    ("10", [59.0, 11.0, 4.0], "biscuits_10", "Biscuit, #10"),
    ("20", [62.0, 13.0, 4.0], "biscuits_20", "Biscuit, #20"),
    ("domino", [21.0, 15.0, 5.0], "dominos", "Domino, 5 x 30 mm"),
];

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
    /// Across the end board, toward its outside face (under a bottom, over a top): where a
    /// pocket screw's pocket goes, and a cam lock's on the other face.
    pub out: [i32; 3],
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
    rows: RefCell<Vec<Row>>,
    slot: usize,         // in SLOTS
    pocket_inside: bool, // the pockets in the end board's inside face
}

/// One pattern for every joint's first cuts that run the same way with the same count: each
/// feature costs time, the more the bigger the design.
struct Row {
    name: String,
    along: [i32; 3],
    count: String,
    pitch: String,
    features: Vec<String>,
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
                "joint_spacing" | "joint_edge" => SPACED.iter().any(|m| uses(m)),
                "dado_depth" => uses("dado"),
                "cam_d" | "cam_bolt" => uses("cam"),
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
                "dado_depth" => (0.5, 25.0),
                "cam_d" => (10.0, 35.0),
                "cam_bolt" => (20.0, 80.0),
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
            size("dado_depth").is_none_or(|g| g <= t.0 - 3.0),
            "`dado_depth` must leave 3 mm of the board behind the groove",
        )?;
        if let (Some(d), Some(bolt)) = (size("cam_d"), size("cam_bolt")) {
            check(
                bolt - 11.0 > d / 2.0 + 3.0 && t.0 >= 12.0,
                "a cam lock needs boards of 12 mm or more and a bolt that reaches past the cam's edge",
            )?;
        }
        check(
            !(uses("confirmat") || uses("screws")) || t.0 < 40.0,
            "boards of 40 mm or more are too thick for 50 mm screws",
        )?;
        let comment = |key: &str| match key {
            "joint_spacing" => "most between two joint fasteners",
            "joint_edge" => "first joint fastener from the board's edge",
            "dowel_d" => "dowel diameter",
            "dado_depth" => "depth of the grooves the boards sit in",
            "cam_d" => "cam lock diameter",
            "cam_bolt" => "cam lock bolt length",
            _ => "dowel length",
        };
        let biscuit = p.get("biscuit").and_then(Value::as_str).unwrap_or("20");
        let slot = (SLOTS.iter().position(|s| s.0 == biscuit))
            .ok_or("`biscuit` must be 0, 10, 20 or domino")?;
        let [_, deep, thick] = SLOTS[slot].1;
        check(
            !uses("biscuit") || (deep <= t.0 - 3.0 && thick <= t.0 - 6.0),
            "the boards are too thin for that biscuit or domino",
        )?;
        let pocket_inside = match p.get("pocket_face").and_then(Value::as_str) {
            None | Some("outside") => false,
            Some("inside") => true,
            _ => return Err("`pocket_face` must be outside or inside".into()),
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
            rows: RefCell::default(),
            slot,
            pocket_inside,
        })
    }

    /// The method for the carcass (`CARCASS`) or the boards inside (`INNER`).
    pub fn method(&self, scope: usize) -> &str {
        &self.methods[scope]
    }

    /// How much longer a board is at each end that sits in a groove (a dado), for `scope`.
    pub fn extension(&self, scope: usize) -> Option<&str> {
        (self.method(scope) == "dado").then(|| self.size("dado_depth"))
    }

    fn size(&self, key: &str) -> &str {
        let found = self.sizes.iter().find(|s| s.0 == key);
        found.map(|s| s.1.as_str()).unwrap_or_default()
    }

    /// Checks that a row of `run_mm` has room for its fasteners: `what` names it.
    pub fn fits(&self, scope: usize, run_mm: f64, p: &Value, what: &str) -> Result<(), String> {
        if !SPACED.contains(&self.method(scope)) {
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
            self.cylinder(&feature, base, json!(axis), &d, &h, targets);
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
            "confirmat" | "screws" => {
                // Through the face board from its outside, then the pilot in the end.
                let [clear, pilot, length] = match method.as_str() {
                    "confirmat" => CONFIRMAT,
                    _ => SCREW,
                };
                let base = shift(&start, neg(j.into), t);
                cut("", base, j.into, mm(clear), t.clone());
                let depth = format!("{} - {t} + 2 mm", mm(length));
                cut("pilot", start.clone(), j.into, mm(pilot), depth);
                if method == "screws" {
                    "screws"
                } else {
                    "confirmats"
                }
            }
            "pocket" => {
                // A screw at 15 degrees to the end board's face, from a pocket in its outside
                // (or inside) face out through the middle of its end into the face board.
                let (cos, sin) = (0.965926, 0.258819);
                let side = if self.pocket_inside { 1.0 } else { -1.0 };
                let axis: Vec<f64> = (0..3)
                    .map(|i| -cos * j.into[i] as f64 + side * sin * j.out[i] as f64 + 0.0)
                    .collect();
                // `s` back along the screw from where it leaves the end
                let at = |s: &str| -> [String; 3] {
                    let mut p = start.clone();
                    for i in 0..3 {
                        if axis[i] > 0.0 {
                            p[i] = format!("{} - ({s}) * {}", p[i], axis[i]);
                        } else if axis[i] < 0.0 {
                            p[i] = format!("{} + ({s}) * {}", p[i], -axis[i]);
                        }
                    }
                    p
                };
                let entry = format!("{t} / 2 / {sin} + 5 mm"); // 5 mm outside the face
                let feature = format!("{name} {method}");
                let base = at(&entry);
                let height = format!("{entry} - 10 mm");
                self.cylinder(
                    &feature,
                    base,
                    json!(axis),
                    "9.5 mm",
                    &height,
                    vec![end.clone()],
                );
                let pilot = format!("{feature} pilot");
                let height = format!("10 mm + min({t} - 3 mm, 15 mm) / {cos}");
                let targets = vec![end.clone(), face.clone()];
                self.cylinder(&pilot, at("10 mm"), json!(axis), "3.5 mm", &height, targets);
                features.extend([feature, pilot]);
                "pocket_screws"
            }
            "cam" => {
                // The bolt's thread in the face board, its shank along the end board's middle
                // to the cam, and the cam's housing from the end board's inside face.
                let (d, bolt) = (self.size("cam_d"), self.size("cam_bolt"));
                let thread = format!("min({t} - 5 mm, 11 mm)");
                cut(
                    "",
                    start.clone(),
                    neg(j.into),
                    "5 mm".into(),
                    thread.clone(),
                );
                let reach = format!("{bolt} - {thread}");
                cut("bolt", start.clone(), j.into, "8 mm".into(), reach.clone());
                let centre = shift(&start, j.into, &reach);
                let face = shift(&centre, neg(j.out), &format!("{t}/2"));
                let depth = format!("min({t} - 3 mm, {d} * 0.85)");
                cut("housing", face, j.out, d.into(), depth);
                "cams"
            }
            "biscuit" => {
                // A slot across the joint, half in each board, centred on the end's thickness.
                let (_, [long, deep, thick], item, _) = SLOTS[self.slot];
                let mut corner = start.clone();
                let mut size = [mm(thick), mm(thick), mm(thick)];
                for i in 0..3 {
                    if j.into[i] != 0 {
                        size[i] = mm(2.0 * deep);
                        corner[i] = format!("{} - {}", start[i], mm(deep));
                    } else if j.along[i] != 0 {
                        size[i] = mm(long);
                        corner[i] = format!("{} - {}", start[i], mm(long / 2.0));
                    } else {
                        corner[i] = format!("{} - {}", start[i], mm(thick / 2.0));
                    }
                }
                let corner = self.kit.at(&corner[0], &corner[1], &corner[2]);
                let feature = format!("{name} {method}");
                self.cuts.borrow_mut().push((
                    "solid.box",
                    json!({
                        "length": size[0], "width": size[1], "height": size[2],
                        "corner": corner, "operation": "cut", "targets": [face, end],
                        "name": feature,
                    }),
                ));
                features.push(feature);
                item
            }
            "dado" => {
                // The end board's own volume where it reaches into the face board: a groove
                // as long as the joint, as wide as the board and `dado_depth` deep.
                let g = self.size("dado_depth");
                let mut corner = j.at.clone();
                let mut size = [t.clone(), t.clone(), t.clone()];
                for i in 0..3 {
                    if j.into[i] != 0 {
                        size[i] = g.to_string();
                        if j.into[i] > 0 {
                            corner[i] = format!("{} - {g}", j.at[i]);
                        }
                    } else if j.along[i] != 0 {
                        size[i] = j.run.clone();
                        if j.along[i] < 0 {
                            corner[i] = format!("{} - {}", j.at[i], j.run);
                        }
                    } else {
                        corner[i] = format!("{} - {t}/2", j.at[i]);
                    }
                }
                let corner = self.kit.at(&corner[0], &corner[1], &corner[2]);
                self.cuts.borrow_mut().push((
                    "solid.box",
                    json!({
                        "length": size[0], "width": size[1], "height": size[2],
                        "corner": corner, "operation": "cut", "targets": [face],
                        "name": format!("{name} dado"),
                    }),
                ));
                return;
            }
            _ => return,
        };
        if j.single {
            self.counts.borrow_mut().push((item, "1".into()));
            return;
        }
        self.counts.borrow_mut().push((item, count.clone()));
        let mut rows = self.rows.borrow_mut();
        match rows
            .iter_mut()
            .find(|r| r.along == j.along && r.count == count)
        {
            Some(row) => row.features.extend(features),
            None => rows.push(Row {
                name: format!("{name} {method} row"),
                along: j.along,
                count,
                pitch,
                features,
            }),
        }
    }

    fn board(&self, part: &str) -> String {
        format!("{} {part}", self.kit.name)
    }

    fn cylinder(
        &self,
        name: &str,
        base: [String; 3],
        axis: Value,
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
        let mut cuts = self.cuts.take();
        for row in self.rows.take() {
            cuts.push((
                "solid.pattern.rectangular",
                json!({
                    "features": row.features, "name": row.name,
                    "dir1": row.along, "count1": row.count, "spacing1": row.pitch,
                }),
            ));
        }
        cuts
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
            "screws" => "Screw, 4 x 50 mm".to_string(),
            "cams" => format!("Cam lock and bolt, {} mm", size("cam_d")),
            "pocket_screws" => "Pocket screw, 32 mm".to_string(),
            slot if slot.starts_with("biscuits") || slot == "dominos" => {
                let found = SLOTS.iter().find(|s| s.2 == slot);
                found.map(|s| s.3).unwrap_or_default().to_string()
            }
            _ => "Confirmat screw, 7 x 50 mm".to_string(),
        };
        items.push((item, qty));
    }
    items
}

#[cfg(test)]
mod tests;

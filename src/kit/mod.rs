//! What every cabinet builder shares: reading sizes, design parameters, panels as named
//! bodies, moving parts as sub-components on joints, and the history folder.
//!
//! A builder writes positions from the front left bottom corner of the cabinet's bounding box;
//! `Kit` shifts them so the cabinet stands on Z = 0, centred on its component's origin.

use crate::Commands;
use serde_json::{json, Value};
use std::cell::RefCell;

const LABEL_HEIGHT: f64 = 20.0; // of a board label's capitals

pub fn num(p: &Value, key: &str, default: f64) -> Result<f64, String> {
    match p.get(key) {
        None | Some(Value::Null) => Ok(default),
        Some(v) => v.as_f64().ok_or(format!("`{key}` must be a number")),
    }
}

/// A constant in an expression: a bare number would be read in the design's unit.
pub fn mm(v: f64) -> String {
    format!("{v} mm")
}

pub fn check(ok: bool, message: &str) -> Result<(), String> {
    if ok {
        Ok(())
    } else {
        Err(message.into())
    }
}

/// What a cabinet's parameters start with: its name, made a valid identifier that is not the
/// hardware's `HW_`.
pub fn prefix(name: &str) -> String {
    let mut prefix: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    // `HW_` is the shared hardware parameters' (`crate::hardware`).
    if !prefix.starts_with(|c: char| c.is_ascii_alphabetic()) || prefix.starts_with("HW_") {
        prefix.insert_str(0, "c_");
    }
    prefix
}

/// `name`, or with the next free number when the design already has a cabinet whose parameters
/// would get the same prefix: Cabinet, Cabinet 2, Cabinet 3; after Base 1 comes Base 2.
/// `design` is `document.inspect`'s reply.
pub fn free_name(name: &str, design: &Value) -> String {
    let taken = |name: &str| {
        let id = format!("{}_width", prefix(name));
        let all = design["params"].as_array().into_iter().flatten();
        all.into_iter().any(|p| p["name"] == id.as_str())
    };
    let name = name.trim();
    if !taken(name) {
        return name.into();
    }
    let (stem, n) = match name.rsplit_once(' ') {
        Some((stem, n)) if n.parse::<u32>().is_ok() => (stem, n.parse().unwrap_or(1)),
        _ => (name, 1),
    };
    (n + 1..)
        .map(|n| format!("{stem} {n}"))
        .find(|candidate| !taken(candidate))
        .unwrap_or_default()
}

/// The sizes a new cabinet takes from the selected one, for `Kit` as `_match` in its input:
/// that cabinet's parameter prefix, and the values in mm. `design` is `document.inspect`'s
/// reply. A cabinet without a plinth has no plinth parameter, and one without a set-back (on
/// legs, or made before it was a field) keeps the dialog's.
pub const MATCHED: [&str; 5] = ["height", "depth", "thickness", "plinth", "plinth_setback"];

///
/// Shared groups (#137): each matched size points at a parameter of the run, `Run_<A>_<key>`,
/// not at the selected cabinet's own. The first match makes it (with the selected cabinet's
/// expression) and points that cabinet's parameter at it too: `setup`, the commands `lib.rs`
/// runs before the build. A cabinet already in a run (its parameter is just a `Run_` name)
/// shares that one. So deleting any cabinet of the run breaks none of the others.
pub fn matched(cabinet: &str, design: &Value) -> Result<Value, String> {
    let from = prefix(cabinet);
    let all: Vec<&Value> = design["params"].as_array().into_iter().flatten().collect();
    let taken = |n: &str| all.iter().any(|p| p["name"] == n);
    let (mut values, mut names, mut setup) =
        (serde_json::Map::new(), serde_json::Map::new(), vec![]);
    for key in MATCHED {
        let id = format!("{from}_{key}");
        let Some(found) = all.iter().find(|p| p["name"] == id.as_str()) else {
            continue;
        };
        let Some(v) = found["value"].as_f64() else {
            continue;
        };
        values.insert(key.into(), json!(v));
        let expression = found["expression"].as_str().unwrap_or_default().trim();
        let group = match expression.starts_with("Run_") && taken(expression) {
            true => expression.to_string(),
            false => {
                let stem = format!("Run_{from}_{key}");
                let group = (1..)
                    .map(|i| {
                        if i == 1 {
                            stem.clone()
                        } else {
                            format!("{stem}_{i}")
                        }
                    })
                    .find(|n| !taken(n))
                    .unwrap_or(stem);
                let comment = format!("shared by the cabinets matched to {cabinet}");
                // An expression over the cabinet's own parameters (`A_height - 300 mm`) would
                // tie the run to that cabinet: the run takes its value instead.
                // ponytail: a value, not the expression rewritten over the run's names.
                let shared = match names_own(expression, &from) {
                    true => format!("{v} mm"),
                    false => expression.to_string(),
                };
                setup.push(json!(["parameters.add", {"name": group, "expression": shared, "comment": comment}]));
                setup.push(json!(["parameters.change", {"name": id, "expression": group}]));
                group
            }
        };
        names.insert(key.into(), json!(group));
    }
    check(
        values.contains_key("height"),
        &format!("`{cabinet}` has no parameters to match"),
    )?;
    Ok(json!({"prefix": from, "values": values, "names": names, "setup": setup}))
}

/// Whether an expression names one of the cabinet `prefix`'s parameters (`A_height`, not
/// `BA_height` or `Tall`).
fn names_own(expression: &str, prefix: &str) -> bool {
    let own = format!("{prefix}_");
    expression.match_indices(&own).any(|(i, _)| {
        let before = expression[..i].chars().next_back();
        !before.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
    })
}

pub struct Kit {
    link: Value,     // `_match`, or null
    line: Value,     // `_line`: a size's expression over a picked line's dimension (#147)
    existing: Value, // `_existing`: the parameters an edited cabinet already has, in mm
    labels: bool,    // write each board's name and size on it
    /// `hardware_models`: on, off or cut (`crate::hardware::model`)
    pub models: &'static str,
    /// `hinge_angle`: how far a door opens, in degrees, the hinge joint's limit
    pub hinge_angle: f64,
    pub name: String,
    pub prefix: String,
    pub unit: &'static str,
    scale: f64, // mm per unit
    centre: RefCell<[String; 2]>,
    commands: RefCell<Commands>,
    bodies: RefCell<Vec<String>>,
    joints: RefCell<Vec<String>>,
    /// A front made of several boards (`fittings::front`): its part and the boards, which every
    /// hole into the part cuts (`targets`).
    pieces: RefCell<Vec<(String, Vec<String>)>>,
}

impl Kit {
    /// Reads the fields every cabinet dialog has: `units` and `name`.
    pub fn new(p: &Value) -> Result<Kit, String> {
        // Sizes come in mm or inches; the parameters keep the unit they were given in.
        let (unit, scale) = match p.get("units").and_then(Value::as_str).unwrap_or("mm") {
            "mm" => ("mm", 1.0),
            "in" => ("in", 25.4),
            _ => return Err("units must be mm or in".into()),
        };
        let name = p.get("name").and_then(Value::as_str);
        let name = name.unwrap_or("Cabinet").trim();
        check(
            !name.is_empty() && name.len() <= 40,
            "name must be 1 to 40 characters",
        )?;
        let models = match p.get("hardware_models").and_then(Value::as_str) {
            None | Some("on") => "on",
            Some("off") => "off",
            Some("cut") => "cut",
            _ => return Err("hardware_models must be on, off or cut".into()),
        };
        let hinge_angle = match p.get("hinge_angle").and_then(Value::as_str) {
            None | Some("110") => 110.0,
            Some("95") => 95.0,
            Some("165") => 165.0,
            _ => return Err("hinge_angle must be 95, 110 or 165".into()),
        };
        Ok(Kit {
            name: name.into(),
            prefix: prefix(name),
            unit,
            scale,
            link: p.get("_match").cloned().unwrap_or_default(),
            line: p.get("_line").cloned().unwrap_or_default(),
            existing: p.get("_existing").cloned().unwrap_or_default(),
            labels: p.get("labels").and_then(Value::as_bool).unwrap_or(false),
            models,
            hinge_angle,
            centre: RefCell::new([mm(0.0), mm(0.0)]),
            commands: RefCell::default(),
            bodies: RefCell::default(),
            joints: RefCell::default(),
            pieces: RefCell::default(),
        })
    }

    /// A size field, in mm whatever unit it was entered in. A matched size is the selected
    /// cabinet's instead, whatever the field says.
    pub fn size(&self, p: &Value, key: &str, default_mm: f64) -> Result<f64, String> {
        if !self.link.is_null() && MATCHED.contains(&key) {
            match self.link["values"][key].as_f64() {
                Some(v) => return Ok(v),
                None if key != "plinth_setback" => return Ok(0.0),
                None => {}
            }
        }
        num(p, key, default_mm / self.scale).map(|v| v * self.scale)
    }

    /// A length in mm as an expression in the dialog's unit: "20 mm", or "1.5 in".
    pub fn length(&self, mm_value: f64) -> String {
        // rounded: 24 in must not come back from mm as 23.999999999999996
        let value = (mm_value / self.scale * 1e6).round() / 1e6;
        format!("{value} {}", self.unit)
    }

    /// Whether an edited cabinet already has the parameter `<Prefix>_<key>`.
    pub fn has(&self, key: &str) -> bool {
        self.existing.get(key).is_some()
    }

    /// Makes a design parameter `<Prefix>_<key>` and returns its name.
    pub fn parameter(&self, key: &str, mm_value: f64, comment: &str) -> String {
        self.parameter_as(key, mm_value, comment, self.length(mm_value))
    }

    /// `parameter` with its expression given: a length that stays in mm whatever the dialog's
    /// unit, such as the reveal.
    pub fn parameter_as(&self, key: &str, mm_value: f64, comment: &str, length: String) -> String {
        let id = format!("{}_{key}", self.prefix);
        // A cabinet that is built again keeps its parameters, and whatever they are linked
        // to: one is only touched when the edit gave it another value.
        if let Some(now) = self.existing.get(key).and_then(Value::as_f64) {
            // A size from a line keeps the line's expression: one already over it differs from
            // the value read only by its rounding (an inch value to 1e-6 in, 1e-5 mm off).
            let over = self.line[key].as_str();
            let tolerance = if over.is_some() { 1e-3 } else { 1e-6 };
            if (now - mm_value).abs() > tolerance {
                let expression = over.map_or(length, String::from);
                self.push((
                    "parameters.change",
                    json!({"name": id, "expression": expression}),
                ));
            }
            return id;
        }
        // A matched size is the run's parameter: change it there, the whole run follows.
        // Width from line: the line's dimension, so the cabinet follows it.
        let expression = match (self.link["values"].get(key), self.line[key].as_str()) {
            // the run's shared parameter (#137), else (an older `_match`) the selected cabinet's
            (Some(_), _) => match self.link["names"][key].as_str() {
                Some(group) => group.to_string(),
                None => format!("{}_{key}", self.link["prefix"].as_str().unwrap_or_default()),
            },
            (None, Some(over)) => over.to_string(),
            (None, None) => length,
        };
        self.push((
            "parameters.add",
            json!({
                "name": id, "expression": expression,
                "comment": format!("{}: {comment}", self.name),
            }),
        ));
        id
    }

    /// The width and depth of the bounding box, as expressions: positions are shifted by half
    /// of each from here on.
    pub fn centre(&self, width: &str, depth: &str) {
        *self.centre.borrow_mut() = [width.into(), depth.into()];
    }

    /// A position counted from the front left bottom corner, as the app gets it.
    pub fn at(&self, x: &str, y: &str, z: &str) -> Value {
        let [w, d] = &*self.centre.borrow();
        json!([format!("{x} - {w}/2"), format!("{y} - {d}/2"), z])
    }

    /// The bodies a cut into `part` works on: the part's board, or a frame's boards.
    pub fn targets(&self, part: &str) -> Vec<String> {
        let pieces = self.pieces.borrow();
        match pieces.iter().find(|(p, _)| p == part) {
            Some((_, boards)) => boards.clone(),
            None => vec![format!("{} {part}", self.name)],
        }
    }

    /// Notes that `part` is made of `boards` (`targets`).
    pub fn pieces(&self, part: &str, boards: Vec<String>) {
        self.pieces.borrow_mut().push((part.to_string(), boards));
    }

    pub fn push(&self, command: (&'static str, Value)) {
        self.commands.borrow_mut().push(command);
    }

    /// A hardware model's box, as `panel` makes a board's but not one of the cabinet's bodies,
    /// so it takes no board material and stays the app's grey (#264).
    pub fn model(&self, part: &str, corner: [&str; 3], size: [&str; 3]) {
        let body = format!("{} {part}", self.name);
        self.push((
            "solid.box",
            json!({
                "length": size[0], "width": size[1], "height": size[2],
                "corner": self.at(corner[0], corner[1], corner[2]), "name": body, "body_name": body,
            }),
        ));
    }

    /// A panel, or with `cut` its volume taken out of those bodies.
    pub fn panel(&self, part: &str, corner: [&str; 3], size: [&str; 3], cut: Option<&[String]>) {
        self.panel_at(part, self.at(corner[0], corner[1], corner[2]), size, cut)
    }

    /// The same, with the corner as the app gets it: for a part built around its own
    /// component's origin.
    pub fn panel_at(&self, part: &str, corner: Value, size: [&str; 3], cut: Option<&[String]>) {
        let body = format!("{} {part}", self.name);
        let mut args = json!({
            "length": size[0], "width": size[1], "height": size[2],
            "corner": corner, "name": body,
        });
        match cut {
            Some(targets) => {
                args["operation"] = json!("cut");
                args["targets"] = json!(targets);
            }
            None => {
                args["body_name"] = json!(body);
                self.bodies.borrow_mut().push(body.clone());
            }
        }
        self.push(("solid.box", args));
        if self.labels && cut.is_none() {
            self.label(&body, &corner, size);
        }
    }

    // ponytail: the name only. The size would have to be text built from parameters, and
    // `number()` takes one parameter, not an expression: two more parameters a board.
    /// A board's name, the one the cut list has, as text in a sketch on its face: in the
    /// middle of it, where it stays when the cabinet changes size. Readable from the front,
    /// from above or from the left, whichever way the board lies. A board is told from its
    /// one size that is a thickness parameter; anything else gets no label.
    fn label(&self, body: &str, corner: &Value, size: [&str; 3]) {
        let thin = |i: &usize| {
            size[*i].starts_with(&self.prefix)
                && size[*i].ends_with("_thickness")
                && !size[*i].contains(' ')
        };
        let [across] = (0..3).filter(thin).collect::<Vec<_>>()[..] else {
            return;
        };
        let at = |i: usize| corner[i].as_str().unwrap_or_default().to_string();
        let mid = |i: usize| format!("{} + ({}) / 2", at(i), size[i]);
        let (origin, x_dir, y_dir) = match across {
            0 => ([at(0), mid(1), mid(2)], [0, -1, 0], [0, 0, 1]),
            1 => ([mid(0), at(1), mid(2)], [1, 0, 0], [0, 0, 1]),
            _ => {
                let top = format!("{} + {}", at(2), size[2]);
                ([mid(0), mid(1), top], [1, 0, 0], [0, 1, 0])
            }
        };
        self.push((
            "sketch.create",
            json!({
                "plane": {"origin": origin, "x_dir": x_dir, "y_dir": y_dir},
                "frame": "component", "name": format!("{body} label"),
            }),
        ));
        self.push((
            "sketch.text",
            json!({
                "text": body, "at": [0, 0], "height": mm(LABEL_HEIGHT),
                "align": "center", "vertical_align": "middle",
            }),
        ));
        self.push(("sketch.finish", json!({})));
    }

    /// A door or a drawer is a component of its own inside the cabinet, held by a joint, so it
    /// can be opened. `open` and `close` go around the commands that build it.
    pub fn open(&self, part: &str) {
        self.push((
            "component.create",
            json!({"name": format!("{} {part}", self.name), "grounded": false}),
        ));
    }

    // ponytail: the angle is a number, so only the position follows the parameters.
    /// For a part that stands at an angle: it is built square around its own component's
    /// origin, then this turns the component about Z and puts its origin at `at`.
    pub fn turn(&self, part: &str, at: Value, degrees: i32) {
        let name = &self.name;
        self.push(("component.activate", json!({"component": name})));
        self.push((
            "occurrence.move",
            json!({
                "occurrence": format!("{name} {part}:1"), "translate": at,
                "axis": [0, 0, 1], "angle": format!("{degrees} deg"),
            }),
        ));
    }

    /// Holds a part that has its own component but does not move.
    pub fn fix(&self, part: &str) {
        let name = &self.name;
        self.push((
            "joint.as_built",
            json!({
                "type": "rigid", "a": format!("{name}:1"), "b": format!("{name} {part}:1"),
                "name": format!("{name} {part} fixed"),
            }),
        ));
    }

    /// A seat: where a cabinet placed against this one's side stands, or this one against
    /// another's. An upright construction axis through a corner of the footprint, `x` and `y`
    /// counted from the front left corner like a panel's, so it follows the parameters.
    /// `side` is L or R, the side it is the seat of; `end` is true at the end of a corner
    /// cabinet's return, where the wall is beside the seat and not behind it.
    ///
    /// A link mates two seats, which holds the cabinets together both ways: side to side and
    /// back to back, whichever of them changes width or depth.
    pub fn seat(&self, side: &str, end: bool, x: &str, y: &str) {
        let [w, d] = self.centre.borrow().clone();
        let at = [format!("{x} - {w}/2"), format!("{y} - {d}/2")];
        let name = seat(&self.name, side, end);
        self.line(&self.name, name, [&at[0], &at[1]], 1);
    }

    /// A hinge line: a construction axis in `component`, upright through `at` (x and y,
    /// expressions in that component's coordinates), that follows the cabinet's parameters.
    /// `turn` is 1 or -1, the way a door on it opens about Z. Returns the axis's name.
    ///
    /// It is where two planes meet that are given outright, not made as features: a plugin
    /// cannot hide what it makes, and planes across every door are a lot to look at. Such a
    /// plane is read in the world's coordinates when it is made and in the component's own
    /// afterwards, so the component must not have been moved or turned yet.
    fn line(&self, component: &str, name: String, [x, y]: [&str; 2], turn: i32) -> String {
        self.push(("component.activate", json!({"component": component})));
        let zero = mm(0.0);
        let across = json!({"plane": {"origin": [x, zero, zero], "normal": [1, 0, 0]}});
        let along = json!({"plane": {"origin": [zero, y, zero], "normal": [0, 1, 0]}});
        // The order of the planes is the way the axis points, and so the way it opens.
        let (a, b) = if turn < 0 {
            (along, across)
        } else {
            (across, along)
        };
        self.push((
            "construct.axis.two_planes",
            json!({"a": a, "b": b, "name": name}),
        ));
        name
    }

    /// The hinge line of a part that is about to be turned (`turn`), in its own coordinates:
    /// made first, while the part still stands square. `hinge` then gets the degrees as `own`.
    pub fn hinge_line(&self, part: &str, joint: &str, at: [&str; 2], turn: i32) {
        let name = &self.name;
        let line = format!("{name} {part} {joint} axis");
        self.line(&format!("{name} {part}"), line, at, turn);
    }

    /// A door's hinge: the part turns about the upright line through `at`, x and y as
    /// expressions in the coordinates of what it hangs on: `on`, the moving part beside it (a
    /// bi-fold door on the first door), or `None` for the cabinet. `own` is for a part that
    /// was turned: the degrees it stands at, its own line made before with `hinge_line`.
    /// `turn` is 1 or -1, the way it opens about Z.
    ///
    /// The line is a construction axis in each of the two components, so the hinge follows
    /// the cabinet's parameters.
    #[allow(clippy::too_many_arguments)]
    pub fn hinge(
        &self,
        part: &str,
        on: Option<&str>,
        joint: &str,
        at: [&str; 2],
        own: Option<i32>,
        turn: i32,
        limit: f64,
    ) {
        let name = &self.name;
        let joint = format!("{name} {part} {joint}");
        let base = match on {
            Some(other) => format!("{name} {other}"),
            None => name.clone(),
        };
        let of_part = match own {
            Some(_) => format!("{joint} axis"),
            None => self.line(&format!("{name} {part}"), format!("{joint} axis"), at, turn),
        };
        let of_base = self.line(&base, format!("{joint} seat axis"), at, turn);
        self.push(("component.activate", json!({"component": name})));
        let mut args = json!({
            "type": "revolute", "name": joint,
            "a": {"occurrence": format!("{base}:1"), "axis": of_base},
            "b": {"occurrence": format!("{name} {part}:1"), "axis": of_part},
        });
        if let Some(degrees) = own {
            // about the hinge's own axis, which points down when it opens the other way
            args["angle"] = json!(format!("{} deg", degrees * turn));
        }
        self.push(("joint.create", args));
        self.push((
            "joint.limits",
            json!({"joint": joint, "min": 0, "max": limit}),
        ));
        self.joints.borrow_mut().push(joint);
    }

    // ponytail: the joint's point is a number, where the part is now. It does not follow a
    // resize, and it is off if the cabinet is made inside a component that has been moved.
    /// `on` is the moving part it hangs on (a bi-fold door on the door beside it), or `None`
    /// for the cabinet itself.
    #[allow(clippy::too_many_arguments)]
    pub fn close(
        &self,
        part: &str,
        on: Option<&str>,
        kind: &str,
        joint: &str,
        point: [f64; 3],
        axis: [i32; 3],
        limit: f64,
    ) {
        let name = &self.name;
        let (occurrence, joint) = (format!("{name} {part}:1"), format!("{name} {part} {joint}"));
        let base = match on {
            Some(other) => format!("{name} {other}:1"),
            None => format!("{name}:1"),
        };
        self.push(("component.activate", json!({"component": name})));
        self.push((
            "joint.as_built",
            json!({
                "type": kind, "a": base, "b": occurrence, "name": joint,
                "at": {"occurrence": occurrence, "point": point, "z": axis},
            }),
        ));
        self.push((
            "joint.limits",
            json!({"joint": joint, "min": 0, "max": limit}),
        ));
        self.joints.borrow_mut().push(joint);
    }

    /// The finished list of commands, with `last` (holes that need every panel in place) at
    /// the end and one folder in the history for the whole cabinet, and the summary.
    /// `size` is the bounding box in mm: width, height, depth.
    pub fn finish(&self, mut last: Commands, size: [f64; 3]) -> (Commands, Value) {
        let mut commands = self.commands.take();
        commands.append(&mut last);
        let features: Vec<&Value> = commands
            .iter()
            .filter(|c| {
                c.0.starts_with("solid.") || c.0.starts_with("construct.") || c.0 == "sketch.create"
            })
            .map(|c| &c.1["name"])
            .collect();
        let group = json!({ "name": self.name, "features": features });
        commands.push(("timeline.group", group));
        let summary = json!({
            "name": self.name,
            "size": { "width": size[0], "height": size[1], "depth": size[2] },
            // the box's width and depth as expressions: Along a line places it by them
            "centre": self.centre.borrow().clone(),
            "bodies": self.bodies.take(),
            "joints": self.joints.take(), // drive them to open doors and drawers
            "parameters": format!("{0}_width, {0}_height, ...", self.prefix),
        });
        (commands, summary)
    }
}

/// The name of a cabinet's seat on its left (L) or right (R) side: see [`Kit::seat`].
pub fn seat(cabinet: &str, side: &str, end: bool) -> String {
    match end {
        true => format!("{cabinet} seat {side} return"),
        false => format!("{cabinet} seat {side}"),
    }
}

#[cfg(test)]
mod tests;

/// The value of the parameter `name` in `document.inspect`'s reply as text, found without parsing
/// it all (a 30-cabinet design's parameters are 78 KB, costly for a dialog hook's 10M fuel).
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub fn param_value(reply: &str, name: &str) -> Option<f64> {
    let quoted = format!("\"{name}\"");
    let mut from = 0;
    while let Some(at) = reply[from..].find(&quoted).map(|i| i + from) {
        from = at + quoted.len();
        let before = reply[..at].trim_end().strip_suffix(':').map(str::trim_end);
        if !before.is_some_and(|b| b.ends_with("\"name\"")) {
            continue;
        }
        // the same parameter's `value` (its keys come sorted: name, unit, value)
        let rest = &reply[from..];
        let end = rest.find("\"name\"").unwrap_or(rest.len());
        let value = rest[..end].find("\"value\"")? + "\"value\"".len();
        let number = rest[value..].trim_start().strip_prefix(':')?.trim_start();
        let len = number
            .find(|c: char| !(c.is_ascii_digit() || "+-.eE".contains(c)))
            .unwrap_or(number.len());
        return number[..len].parse().ok();
    }
    None
}

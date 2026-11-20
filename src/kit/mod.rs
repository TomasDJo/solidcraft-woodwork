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

/// What a cabinet's parameters start with: its name, made a valid identifier.
pub fn prefix(name: &str) -> String {
    let mut prefix: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    if !prefix.starts_with(|c: char| c.is_ascii_alphabetic()) {
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
/// reply. A cabinet without a plinth has no plinth parameter.
pub const MATCHED: [&str; 4] = ["height", "depth", "thickness", "plinth"];

pub fn matched(cabinet: &str, design: &Value) -> Result<Value, String> {
    let from = prefix(cabinet);
    let mut values = serde_json::Map::new();
    for key in MATCHED {
        let id = format!("{from}_{key}");
        let all = design["params"].as_array().into_iter().flatten();
        let found = all.into_iter().find(|p| p["name"] == id.as_str());
        if let Some(v) = found.and_then(|p| p["value"].as_f64()) {
            values.insert(key.into(), json!(v));
        }
    }
    check(
        values.contains_key("height"),
        &format!("`{cabinet}` has no parameters to match"),
    )?;
    Ok(json!({"prefix": from, "values": values}))
}

pub struct Kit {
    link: Value,     // `_match`, or null
    existing: Value, // `_existing`: the parameters an edited cabinet already has, in mm
    labels: bool,    // write each board's name and size on it
    pub name: String,
    pub prefix: String,
    unit: &'static str,
    scale: f64, // mm per unit
    centre: RefCell<[String; 2]>,
    commands: RefCell<Commands>,
    bodies: RefCell<Vec<String>>,
    joints: RefCell<Vec<String>>,
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
        Ok(Kit {
            name: name.into(),
            prefix: prefix(name),
            unit,
            scale,
            link: p.get("_match").cloned().unwrap_or_default(),
            existing: p.get("_existing").cloned().unwrap_or_default(),
            labels: p.get("labels").and_then(Value::as_bool).unwrap_or(false),
            centre: RefCell::new([mm(0.0), mm(0.0)]),
            commands: RefCell::default(),
            bodies: RefCell::default(),
            joints: RefCell::default(),
        })
    }

    /// A size field, in mm whatever unit it was entered in. A matched size is the selected
    /// cabinet's instead, whatever the field says.
    pub fn size(&self, p: &Value, key: &str, default_mm: f64) -> Result<f64, String> {
        if !self.link.is_null() && MATCHED.contains(&key) {
            return Ok(self.link["values"][key].as_f64().unwrap_or(0.0));
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
        let id = format!("{}_{key}", self.prefix);
        // A cabinet that is built again keeps its parameters, and whatever they are linked
        // to: one is only touched when the edit gave it another value.
        if let Some(now) = self.existing.get(key).and_then(Value::as_f64) {
            if (now - mm_value).abs() > 1e-6 {
                let expression = self.length(mm_value);
                self.push((
                    "parameters.change",
                    json!({"name": id, "expression": expression}),
                ));
            }
            return id;
        }
        // A matched size is the selected cabinet's parameter: change it there, both follow.
        let expression = match self.link["values"].get(key) {
            Some(_) => format!("{}_{key}", self.link["prefix"].as_str().unwrap_or_default()),
            None => self.length(mm_value),
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

    pub fn push(&self, command: (&'static str, Value)) {
        self.commands.borrow_mut().push(command);
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

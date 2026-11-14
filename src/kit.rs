//! What every cabinet builder shares: reading sizes, design parameters, panels as named
//! bodies, moving parts as sub-components on joints, and the history folder.
//!
//! A builder writes positions from the front left bottom corner of the cabinet's bounding box;
//! `Kit` shifts them so the cabinet stands on Z = 0, centred on its component's origin.

use crate::Commands;
use serde_json::{json, Value};
use std::cell::RefCell;

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

pub struct Kit {
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
            centre: RefCell::new([mm(0.0), mm(0.0)]),
            commands: RefCell::default(),
            bodies: RefCell::default(),
            joints: RefCell::default(),
        })
    }

    /// A size field, in mm whatever unit it was entered in.
    pub fn size(&self, p: &Value, key: &str, default_mm: f64) -> Result<f64, String> {
        num(p, key, default_mm / self.scale).map(|v| v * self.scale)
    }

    /// Makes a design parameter `<Prefix>_<key>` and returns its name.
    pub fn parameter(&self, key: &str, mm_value: f64, comment: &str) -> String {
        let id = format!("{}_{key}", self.prefix);
        // rounded: 24 in must not come back from mm as 23.999999999999996
        let value = (mm_value / self.scale * 1e6).round() / 1e6;
        self.push((
            "parameters.add",
            json!({
                "name": id, "expression": format!("{value} {}", self.unit),
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
        let body = format!("{} {part}", self.name);
        let mut args = json!({
            "length": size[0], "width": size[1], "height": size[2],
            "corner": self.at(corner[0], corner[1], corner[2]), "name": body,
        });
        match cut {
            Some(targets) => {
                args["operation"] = json!("cut");
                args["targets"] = json!(targets);
            }
            None => {
                args["body_name"] = json!(body);
                self.bodies.borrow_mut().push(body);
            }
        }
        self.push(("solid.box", args));
    }

    /// A door or a drawer is a component of its own inside the cabinet, held by a joint, so it
    /// can be opened. `open` and `close` go around the commands that build it.
    pub fn open(&self, part: &str) {
        self.push((
            "component.create",
            json!({"name": format!("{} {part}", self.name), "grounded": false}),
        ));
    }

    // ponytail: the joint's point is a number, where the part is now. It does not follow a
    // resize, and it is off if the cabinet is made inside a component that has been moved.
    pub fn close(
        &self,
        part: &str,
        kind: &str,
        joint: &str,
        point: [f64; 3],
        axis: [i32; 3],
        limit: f64,
    ) {
        let name = &self.name;
        let (occurrence, joint) = (format!("{name} {part}:1"), format!("{name} {part} {joint}"));
        self.push(("component.activate", json!({"component": name})));
        self.push((
            "joint.as_built",
            json!({
                "type": kind, "a": format!("{name}:1"), "b": occurrence, "name": joint,
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
            .filter(|c| c.0.starts_with("solid."))
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

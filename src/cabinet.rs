//! Box carcass. X is width, Y is depth (front at 0), Z is height.
//!
//! The sizes become design parameters (`Name_width`, ...) and every panel and hole is an
//! expression over them, so the cabinet follows when a parameter is changed later. The numbers
//! in here only check the input; the model itself is all expressions.

use crate::Commands;
use serde_json::{json, Value};
use std::cell::RefCell;

const GROOVE_INSET: f64 = 12.0; // back groove, from the rear edge
const KICK_RECESS: f64 = 50.0; // plinth board, back from the front
const PITCH: f64 = 32.0; // 32 mm system: shelf pin holes
const PIN_SETBACK: f64 = 37.0; // pin rows, from the front and from the back of the shelf
const PIN_D: f64 = 5.0;
const DOOR_GAP: f64 = 2.0; // around overlay doors, and between door and carcass
const CUP_D: f64 = 35.0; // hinge cup
const CUP_DEPTH: f64 = 13.0;
const CUP_CENTRE: f64 = 22.5; // from the door edge: 5 mm of wood outside the cup
const CUP_FROM_END: f64 = 100.0; // from the door's top and bottom
const SLIDE: f64 = 12.7; // ball-bearing drawer slide, each side
const BOX_T: f64 = 12.0; // drawer box sides, front and back
const BOX_BOTTOM: f64 = 6.0;
const BOX_LOWER: f64 = 40.0; // the box is this much lower than its front
const BOX_LIFT: f64 = 15.0; // and starts this far above the front's lower edge
const BOX_BEHIND: f64 = 10.0; // room behind the box

fn num(p: &Value, key: &str, default: f64) -> Result<f64, String> {
    match p.get(key) {
        None | Some(Value::Null) => Ok(default),
        Some(v) => v.as_f64().ok_or(format!("`{key}` must be a number")),
    }
}

/// A constant in an expression: a bare number would be read in the design's unit.
fn mm(v: f64) -> String {
    format!("{v} mm")
}

fn check(ok: bool, message: &str) -> Result<(), String> {
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

pub fn build(p: &Value) -> Result<(Commands, Value), String> {
    // Sizes come in mm or inches; the parameters keep the unit they were given in.
    let (unit, scale) = match p.get("units").and_then(Value::as_str).unwrap_or("mm") {
        "mm" => ("mm", 1.0),
        "in" => ("in", 25.4),
        _ => return Err("units must be mm or in".into()),
    };
    let size = |key: &str, default_mm: f64| num(p, key, default_mm / scale).map(|v| v * scale);
    let (w, h, d) = (
        size("width", 600.0)?,
        size("height", 720.0)?,
        size("depth", 560.0)?,
    );
    let t = size("thickness", 18.0)?;
    let bt = size("back_thickness", 6.0)?;
    let plinth = size("plinth", 0.0)?;
    let shelves = num(p, "shelves", 1.0)?;
    let doors = num(p, "doors", 0.0)?;
    let dividers = num(p, "dividers", 0.0)?;
    let pins = p
        .get("shelf_pins")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let drawers = num(p, "drawers", 0.0)?;
    check(
        (0.0..=8.0).contains(&drawers) && drawers.fract() == 0.0,
        "drawers must be 0 to 8",
    )?;
    // A drawer cabinet is all drawers: no shelves behind them.
    let (shelves, pins) = if drawers > 0.0 {
        (0.0, false)
    } else {
        (shelves, pins)
    };
    let back = p.get("back").and_then(Value::as_str).unwrap_or("groove");
    let name = p
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("Cabinet")
        .trim();

    check((3.0..=50.0).contains(&t), "thickness must be 3 to 50 mm")?;
    check(
        (2.0..=t).contains(&bt),
        "back thickness must be 2 mm up to the panel thickness",
    )?;
    check(
        (0.0..=20.0).contains(&shelves) && shelves.fract() == 0.0,
        "shelves must be 0 to 20",
    )?;
    check(plinth >= 0.0, "plinth cannot be negative")?;
    check(
        matches!(back, "none" | "overlay" | "groove"),
        "back must be none, overlay or groove",
    )?;
    check(
        !name.is_empty() && name.len() <= 40,
        "name must be 1 to 40 characters",
    )?;
    check(
        w <= 5000.0 && h <= 5000.0 && d <= 5000.0,
        "sizes are limited to 5000 mm",
    )?;

    let inner_w = w - 2.0 * t;
    let clear_h = h - plinth - 2.0 * t; // between bottom and top panels
    let gap = (clear_h - shelves * t) / (shelves + 1.0);
    check(
        (0.0..=10.0).contains(&dividers) && dividers.fract() == 0.0,
        "dividers must be 0 to 10",
    )?;
    // Dividers split the inside into equal compartments.
    let cell_w = (inner_w - dividers * t) / (dividers + 1.0);
    check(
        cell_w >= 50.0,
        "width is too small for the panels and dividers",
    )?;
    // The app runs at most 256 commands per plugin call.
    check(
        (dividers + 1.0) * shelves <= 200.0,
        "too many shelves: 200 at most in one cabinet",
    )?;
    check(
        gap >= 20.0,
        "height is too small for the plinth and shelves",
    )?;
    check(d >= 150.0 + t, "depth is too small")?;
    // More than two doors need a divider behind every hinge.
    check(
        [0.0, 1.0, 2.0, dividers + 1.0].contains(&doors),
        "doors must be 0, 1, 2 or one per compartment (dividers + 1)",
    )?;
    check(
        drawers == 0.0 || (doors == 0.0 && dividers == 0.0),
        "drawers cannot be combined with doors or dividers",
    )?;
    // Height of one drawer front, and width of the box between the slides.
    let front_h = (h - plinth - (drawers + 1.0) * DOOR_GAP) / drawers.max(1.0);
    check(
        drawers == 0.0 || front_h >= BOX_LOWER + 50.0,
        "too low for that many drawers",
    )?;
    check(
        drawers == 0.0 || inner_w - 2.0 * SLIDE >= 100.0,
        "too narrow for drawers",
    )?;
    check(
        doors == 0.0 || t >= 16.0,
        "doors with hinge cups need panels of 16 mm or more",
    )?;
    check(
        doors == 0.0 || h - plinth >= 300.0,
        "too low for a door with two hinges",
    )?;
    check(
        !pins || t >= 12.0,
        "shelf pin holes need panels of 12 mm or more",
    )?;
    // Pin holes sit on a 32 mm grid above the bottom panel, two pitches clear of both ends.
    check(
        !pins || clear_h >= 4.0 * PITCH,
        "too low for shelf pin holes",
    )?;
    let shelf_d = match back {
        "overlay" => d - bt,
        "groove" => d - GROOVE_INSET - bt,
        _ => d,
    };

    let prefix = prefix(name);
    let mut commands = Vec::new();
    let mut parameter = |key: &str, mm_value: f64, comment: &str| {
        let id = format!("{prefix}_{key}");
        commands.push((
            "parameters.add",
            json!({
                // rounded: 24 in must not come back from mm as 23.999999999999996
                "name": id, "expression": format!("{} {unit}", (mm_value / scale * 1e6).round() / 1e6),
                "comment": format!("{name}: {comment}"),
            }),
        ));
        id
    };
    let ew = parameter("width", w, "outside width");
    let eh = parameter("height", h, "outside height, plinth included");
    let ed = parameter("depth", d, "outside depth, back included");
    let et = parameter("thickness", t, "panel thickness");
    let ebt = match back {
        "none" => mm(0.0),
        _ => parameter("back_thickness", bt, "back panel thickness"),
    };
    let epl = match plinth > 0.0 {
        true => parameter("plinth", plinth, "plinth height"),
        false => mm(0.0),
    };

    // Derived sizes, as expressions.
    let n = dividers as u32;
    let s = shelves as u32;
    let eiw = format!("({ew} - 2*{et})"); // inner width
    let ecell = format!("(({eiw} - {n}*{et}) / {})", n + 1); // one compartment
    let ech = format!("({eh} - {epl} - 2*{et})"); // clear height
    let egap = format!("(({ech} - {s}*{et}) / {})", s + 1); // between shelves
    let elast = format!("floor(({ech} - {}) / {})", mm(2.0 * PITCH), mm(PITCH)); // top pin row
                                                                                 // Depth of the sides, top and bottom, and of the shelves (which stop at the back).
    let (eside_d, eshelf_d) = match back {
        "overlay" => (format!("({ed} - {ebt})"), format!("({ed} - {ebt})")),
        "groove" => (ed.clone(), format!("({ed} - {} - {ebt})", mm(GROOVE_INSET))),
        _ => (ed.clone(), ed.clone()),
    };

    let commands = RefCell::new(commands);
    let push = |command: (&'static str, Value)| commands.borrow_mut().push(command);
    let mut joints = Vec::new();
    let mut bodies = Vec::new();
    let mut drill = Vec::new(); // holes, run after the panels exist
    let carcass = ["Side L", "Side R", "Bottom", "Top"].map(|part| format!("{name} {part}"));
    // The cabinet stands on Z = 0, centred on the origin, so the view orbits around it:
    // positions are written from the front left bottom corner and shifted here.
    let place =
        |x: &str, y: &str, z: &str| json!([format!("{x} - {ew}/2"), format!("{y} - {ed}/2"), z]);
    // A panel, or with `cut` its volume taken out of the carcass panels.
    let mut panel = |part: &str, corner: [&str; 3], size: [&str; 3], cut: bool| {
        let body = format!("{name} {part}");
        let mut args = json!({
            "length": size[0], "width": size[1], "height": size[2],
            "corner": place(corner[0], corner[1], corner[2]), "name": body,
        });
        if cut {
            args["operation"] = json!("cut");
            args["targets"] = json!(carcass);
        } else {
            args["body_name"] = json!(body);
            bodies.push(body);
        }
        push(("solid.box", args));
    };
    // A door or a drawer is a component of its own inside the cabinet, held by a joint, so it
    // can be opened. `open` and `close` go around the commands that build it.
    let cabinet_occurrence = format!("{name}:1");
    let open = |part: &str| {
        push((
            "component.create",
            json!({"name": format!("{name} {part}"), "grounded": false}),
        ))
    };
    // ponytail: the joint's point is a number, where the part is now. It does not follow a
    // resize, and it is off if the cabinet is made inside a component that has been moved.
    let mut close =
        |part: &str, kind: &str, joint: &str, point: [f64; 3], axis: [i32; 3], limit: f64| {
            let (occurrence, joint) =
                (format!("{name} {part}:1"), format!("{name} {part} {joint}"));
            push(("component.activate", json!({"component": name})));
            push((
                "joint.as_built",
                json!({
                    "type": kind, "a": cabinet_occurrence, "b": occurrence, "name": joint,
                    "at": {"occurrence": occurrence, "point": point, "z": axis},
                }),
            ));
            push((
                "joint.limits",
                json!({"joint": joint, "min": 0, "max": limit}),
            ));
            joints.push(joint);
        };

    let zero = mm(0.0);
    let bottom_top = format!("{epl} + {et}"); // top face of the bottom panel
    panel("Side L", [&zero, &zero, &zero], [&et, &eside_d, &eh], false);
    panel(
        "Side R",
        [&format!("{ew} - {et}"), &zero, &zero],
        [&et, &eside_d, &eh],
        false,
    );
    panel("Bottom", [&et, &zero, &epl], [&eiw, &eside_d, &et], false);
    panel(
        "Top",
        [&et, &zero, &format!("{eh} - {et}")],
        [&eiw, &eside_d, &et],
        false,
    );
    // Left face of each divider. They stand between bottom and top, as deep as the shelves.
    let divider_x: Vec<String> = (1..=n).map(|k| format!("({ecell} + {et}) * {k}")).collect();
    for (k, x) in divider_x.iter().enumerate() {
        let part = format!("Divider {}", k + 1);
        panel(
            &part,
            [x, &zero, &bottom_top],
            [&et, &eshelf_d, &ech],
            false,
        );
    }
    // The same shelves in every compartment.
    for cell in 0..=n {
        for i in 0..s {
            // Height over the bottom panel: evenly spaced, or on the nearest row of pins.
            let mut over = format!("{egap}*{} + {et}*{i}", i + 1);
            if pins {
                let (pitch, r) = (mm(PITCH), mm(PIN_D / 2.0));
                over = format!(
                    "min(max(round(({over} - {r}) / {pitch}), 2), {elast}) * {pitch} + {r}"
                );
            }
            let part = match n {
                0 => format!("Shelf {}", i + 1),
                _ => format!("Shelf {}.{}", cell + 1, i + 1), // compartment.shelf
            };
            let x = format!("{et} + ({ecell} + {et}) * {cell}");
            let z = format!("{bottom_top} + {over}");
            panel(&part, [&x, &zero, &z], [&ecell, &eshelf_d, &et], false);
        }
    }
    if plinth > 0.0 {
        panel(
            "Plinth",
            [&et, &mm(KICK_RECESS), &zero],
            [&eiw, &et, &epl],
            false,
        );
    }
    match back {
        "overlay" => {
            let y = format!("{ed} - {ebt}");
            panel(
                "Back",
                [&zero, &y, &epl],
                [&ew, &ebt, &format!("{eh} - {epl}")],
                false,
            )
        }
        "groove" => {
            // Cut the back panel's own volume out of the carcass first: stopped grooves
            // that fit exactly.
            let g = format!("min({et}/2, {})", mm(10.0)); // groove depth
            let corner = [
                format!("{et} - {g}"),
                format!("{ed} - {} - {ebt}", mm(GROOVE_INSET)),
                format!("{bottom_top} - {g}"),
            ];
            let corner = [corner[0].as_str(), &corner[1], &corner[2]];
            let size = [
                format!("{eiw} + 2*{g}"),
                ebt.clone(),
                format!("{ech} + 2*{g}"),
            ];
            let size = [size[0].as_str(), &size[1], &size[2]];
            panel("Back groove", corner, size, true);
            panel("Back", corner, size, false);
        }
        _ => {}
    }

    if pins {
        // One hole per panel, repeated up the panel and across to the rear row. Blind in the
        // sides; through a divider, which carries shelves on both faces.
        let blind = format!("min({et} - {}, {})", mm(5.0), mm(12.0));
        let mut rows = vec![
            ("Side L".to_string(), et.clone(), blind.clone()),
            ("Side R".to_string(), format!("{ew} - {et}"), blind),
        ];
        for (k, x) in divider_x.iter().enumerate() {
            rows.push((format!("Divider {}", k + 1), x.clone(), et.clone()));
        }
        for (part, x, depth) in rows {
            let feature = format!("{name} {part} pins");
            let z = format!("{bottom_top} + {}", mm(2.0 * PITCH));
            drill.push((
                "solid.hole",
                json!({
                    "position": place(&x, &mm(PIN_SETBACK), &z),
                    "diameter": mm(PIN_D), "depth": depth,
                    "targets": [format!("{name} {part}")], "name": feature,
                }),
            ));
            drill.push((
                "solid.pattern.rectangular",
                json!({
                    "features": [feature], "name": format!("{name} {part} pin rows"),
                    "dir1": [0, 0, 1], "count1": format!("{elast} - 1"), "spacing1": mm(PITCH),
                    "dir2": [0, 1, 0], "count2": 2,
                    "spacing2": format!("{eshelf_d} - {}", mm(2.0 * PIN_SETBACK)),
                }),
            ));
        }
    }

    // Equal overlay doors across the front. They hinge left, right, left... so pairs open from
    // the middle. One door per compartment puts every gap on a divider (within half a panel).
    let count = doors as usize;
    let gap_mm = mm(DOOR_GAP);
    let door_w = format!(
        "(({ew} - {} - {}*{gap_mm}) / {count})",
        mm(2.0 * DOOR_GAP),
        count.max(1) - 1
    );
    let door_h = format!("{eh} - {epl} - {}", mm(2.0 * DOOR_GAP));
    let (z0, z1) = (format!("{epl} + {gap_mm}"), format!("{eh} - {gap_mm}"));
    for i in 0..count {
        let part = match count {
            1 => "Door".to_string(),
            2 => ["Door L", "Door R"][i].to_string(),
            _ => format!("Door {}", i + 1),
        };
        let x0 = format!("{gap_mm} + ({door_w} + {gap_mm}) * {i}");
        let y = format!("-{gap_mm} - {et}");
        open(&part);
        panel(&part, [&x0, &y, &z0], [&door_w, &et, &door_h], false);
        let cup_x = match i % 2 {
            0 => format!("{x0} + {}", mm(CUP_CENTRE)),
            _ => format!("{x0} + {door_w} - {}", mm(CUP_CENTRE)),
        };
        let mut cups = vec![
            format!("{z0} + {}", mm(CUP_FROM_END)),
            format!("{z1} - {}", mm(CUP_FROM_END)),
        ];
        if h - plinth - 2.0 * DOOR_GAP > 1000.0 {
            cups.push(format!("({z0} + {z1}) / 2"));
        }
        for (j, z) in cups.iter().enumerate() {
            push((
                "solid.hole",
                json!({
                    "position": place(&cup_x, &format!("-{gap_mm}"), z), "direction": [0, -1, 0],
                    "diameter": mm(CUP_D), "depth": mm(CUP_DEPTH),
                    "targets": [format!("{name} {part}")],
                    "name": format!("{name} {part} hinge cup {}", j + 1),
                }),
            ));
        }
        // The hinge: the door's inner edge on its hinge side, turning outwards up to 110 degrees.
        let width = (w - 2.0 * DOOR_GAP - (doors - 1.0) * DOOR_GAP) / doors;
        let left = -w / 2.0 + DOOR_GAP + (width + DOOR_GAP) * i as f64;
        let (x, axis) = if i % 2 == 0 {
            (left, -1)
        } else {
            (left + width, 1)
        };
        close(
            &part,
            "revolute",
            "hinge",
            [x, -d / 2.0 - DOOR_GAP, plinth + DOOR_GAP],
            [0, 0, axis],
            110.0,
        );
    }
    // A stack of equal drawers: an overlay front, and behind it a box that runs on slides.
    let count = drawers as usize;
    let front_mm = front_h;
    let front_h = format!(
        "(({eh} - {epl} - {}*{gap_mm}) / {})",
        count + 1,
        count.max(1)
    );
    let box_w = format!("({eiw} - {})", mm(2.0 * SLIDE));
    let box_d = format!("({eshelf_d} - {})", mm(BOX_BEHIND));
    let box_h = format!("({front_h} - {})", mm(BOX_LOWER));
    let (bt, back_y) = (mm(BOX_T), format!("-{gap_mm} + {box_d} - {}", mm(BOX_T)));
    for i in 0..count {
        let drawer = format!("Drawer {}", i + 1);
        let z = format!("{epl} + {gap_mm} + ({front_h} + {gap_mm}) * {i}"); // front's lower edge
        let front_w = format!("{ew} - {}", mm(2.0 * DOOR_GAP));
        let y = format!("-{gap_mm} - {et}");
        open(&drawer);
        panel(
            &format!("{drawer} Front"),
            [&gap_mm, &y, &z],
            [&front_w, &et, &front_h],
            false,
        );
        // The box touches the back of the front. Its sides run the full depth; front, back
        // and bottom sit between them.
        let (x, y, z) = (
            format!("{et} + {}", mm(SLIDE)),
            format!("-{gap_mm}"),
            format!("{z} + {}", mm(BOX_LIFT)),
        );
        let right = format!("{x} + {box_w} - {bt}");
        let inner_x = format!("{x} + {bt}");
        let inner_w = format!("{box_w} - {}", mm(2.0 * BOX_T));
        let inner_y = format!("{y} + {bt}");
        let inner_d = format!("{box_d} - {}", mm(2.0 * BOX_T));
        panel(
            &format!("{drawer} Left"),
            [&x, &y, &z],
            [&bt, &box_d, &box_h],
            false,
        );
        panel(
            &format!("{drawer} Right"),
            [&right, &y, &z],
            [&bt, &box_d, &box_h],
            false,
        );
        panel(
            &format!("{drawer} Box front"),
            [&inner_x, &y, &z],
            [&inner_w, &bt, &box_h],
            false,
        );
        panel(
            &format!("{drawer} Box back"),
            [&inner_x, &back_y, &z],
            [&inner_w, &bt, &box_h],
            false,
        );
        panel(
            &format!("{drawer} Bottom"),
            [&inner_x, &inner_y, &z],
            [&inner_w, &inner_d, &mm(BOX_BOTTOM)],
            false,
        );
        // The slide: straight out of the front, as far as four fifths of the box.
        let at = [
            0.0,
            -d / 2.0 - DOOR_GAP,
            plinth + DOOR_GAP + (front_mm + DOOR_GAP) * i as f64,
        ];
        let travel = ((shelf_d - BOX_BEHIND) * 0.8).round();
        close(&drawer, "slider", "slide", at, [0, -1, 0], travel);
    }
    let mut commands = commands.into_inner();
    commands.append(&mut drill);

    // One folder in the history for the whole cabinet.
    let features: Vec<&Value> = commands
        .iter()
        .filter(|c| c.0.starts_with("solid."))
        .map(|c| &c.1["name"])
        .collect();
    let group = json!({ "name": name, "features": features });
    commands.push(("timeline.group", group));

    let summary = json!({
        "name": name,
        "size": { "width": w, "height": h, "depth": d }, // mm
        "bodies": bodies,
        "joints": joints, // drive them to open doors and drawers
        "parameters": format!("{prefix}_width, {prefix}_height, ..."),
        // one compartment
        "inner": { "width": cell_w, "height": clear_h, "depth": shelf_d },
    });
    Ok((commands, summary))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count(commands: &[(&str, Value)], id: &str) -> usize {
        commands.iter().filter(|c| c.0 == id).count()
    }

    #[test]
    fn default_cabinet() {
        let (commands, summary) = build(&json!({})).unwrap();
        // width, height, depth, thickness, back thickness
        assert_eq!(count(&commands, "parameters.add"), 5);
        assert_eq!(commands[0].1["name"], "Cabinet_width");
        assert_eq!(commands[0].1["expression"], "600 mm");
        // 2 sides, bottom, top, 1 shelf, groove cut, back
        assert_eq!(count(&commands, "solid.box"), 7);
        assert_eq!(summary["bodies"].as_array().unwrap().len(), 6);
        assert_eq!(summary["inner"]["width"], 564.0);
        // every size is an expression over the parameters
        let side = &commands[5].1;
        assert_eq!(side["height"], "Cabinet_height");
        assert_eq!(side["corner"][0], "0 mm - Cabinet_width/2");
        // the history folder holds every feature
        let group = &commands.last().unwrap().1;
        assert_eq!(group["features"].as_array().unwrap().len(), 7);
    }

    #[test]
    fn inches_stay_inches() {
        let p = json!({"units": "in", "width": 24, "height": 30, "depth": 22,
                       "thickness": 0.75, "back_thickness": 0.25, "name": "Base 1"});
        let (commands, summary) = build(&p).unwrap();
        assert_eq!(commands[0].1["name"], "Base_1_width");
        assert_eq!(commands[0].1["expression"], "24 in");
        assert_eq!(summary["inner"]["width"], 24.0 * 25.4 - 2.0 * 19.05);
    }

    #[test]
    fn options_add_parts() {
        let p = json!({"back": "overlay", "shelves": 0, "plinth": 100});
        let (commands, _) = build(&p).unwrap();
        assert_eq!(count(&commands, "parameters.add"), 6); // with the plinth
        assert_eq!(count(&commands, "solid.box"), 6); // 4 panels, plinth, back

        let p = json!({"shelf_pins": true, "doors": 2, "height": 1200});
        let (commands, summary) = build(&p).unwrap();
        assert_eq!(count(&commands, "solid.pattern.rectangular"), 2);
        assert_eq!(count(&commands, "solid.hole"), 2 + 2 * 3); // pin seeds, 3 cups per tall door
        assert_eq!(summary["bodies"].as_array().unwrap().len(), 8);
    }

    #[test]
    fn drawers_replace_shelves() {
        let (commands, summary) =
            build(&json!({"drawers": 3, "shelves": 2, "plinth": 100})).unwrap();
        let named = |n: &str| {
            commands
                .iter()
                .any(|c| c.1["name"] == format!("Cabinet {n}"))
        };
        assert!(named("Drawer 3 Front") && named("Drawer 3 Bottom") && !named("Shelf 1"));
        // 4 carcass, plinth, back, 3 drawers of 6 parts
        assert_eq!(summary["bodies"].as_array().unwrap().len(), 6 + 3 * 6);
        assert!(build(&json!({"drawers": 2, "doors": 1})).is_err());
        assert!(build(&json!({"drawers": 8, "height": 400})).is_err());
    }

    #[test]
    fn dividers_make_equal_compartments() {
        let p = json!({"width": 1200, "dividers": 2, "shelves": 3, "shelf_pins": true});
        let (commands, summary) = build(&p).unwrap();
        // (1200 - 4 panels of 18) / 3
        assert_eq!(summary["inner"]["width"], 376.0);
        let named = |n: &str| {
            commands
                .iter()
                .find(|c| c.1["name"] == format!("Cabinet {n}"))
        };
        assert!(named("Shelf 3.3").is_some());
        assert!(named("Shelf 4.1").is_none());
        // pins: blind in the sides, through the dividers
        assert_eq!(
            named("Divider 1 pins").unwrap().1["depth"],
            "Cabinet_thickness"
        );
        assert!(named("Side L pins").unwrap().1["depth"]
            .as_str()
            .unwrap()
            .starts_with("min("));
        // 4 carcass, 2 dividers, 9 shelves, back
        assert_eq!(summary["bodies"].as_array().unwrap().len(), 16);
    }

    #[test]
    fn refuses_bad_input() {
        assert!(build(&json!({"width": 300, "dividers": 5})).is_err());
        assert!(build(&json!({"width": 60})).is_err());
        assert!(build(&json!({"height": 200, "shelves": 8})).is_err());
        assert!(build(&json!({"back": "glass"})).is_err());
        assert!(build(&json!({"width": "wide"})).is_err());
        assert!(build(&json!({"units": "cm"})).is_err());
        assert!(build(&json!({"doors": 3})).is_err());
        assert!(build(&json!({"doors": 1, "thickness": 12})).is_err());
        // one door per compartment
        let (commands, _) = build(&json!({"width": 908, "dividers": 2, "doors": 3})).unwrap();
        assert!(commands.iter().any(|c| c.1["name"] == "Cabinet Door 3"));
    }
}

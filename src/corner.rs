//! L-shaped corner cabinet: one leg along the back wall, the other (the return) running
//! forward along a side wall.
//!
//! Written for a corner at the back left: the footprint's bounding box is `width` (X) by
//! `return` (Y, the front at 0), and the return is on the left. A right corner is the same
//! mirrored in X. Like the box cabinet, every size is an expression over design parameters.

use crate::kit::{check, mm, num, Kit};
use crate::Commands;
use serde_json::{json, Value};

const KICK_RECESS: f64 = 50.0; // plinth boards, back from the fronts
const DOOR_GAP: f64 = 2.0;
const CUP_D: f64 = 35.0; // hinge cup
const CUP_DEPTH: f64 = 13.0;
const CUP_CENTRE: f64 = 22.5; // from the door's hinge edge
const CUP_FROM_END: f64 = 100.0; // from the door's top and bottom

pub fn build(p: &Value) -> Result<(Commands, Value), String> {
    let kit = Kit::new(p)?;
    let name = kit.name.as_str();
    let size = |key: &str, default_mm: f64| kit.size(p, key, default_mm);
    let (a, b, d, h) = (
        size("width", 900.0)?,
        size("return", 900.0)?,
        size("depth", 560.0)?,
        size("height", 720.0)?,
    );
    let t = size("thickness", 18.0)?;
    let plinth = size("plinth", 0.0)?;
    let shelves = num(p, "shelves", 1.0)?;
    let doors = p.get("doors").and_then(Value::as_bool).unwrap_or(false);
    let right = match p.get("corner").and_then(Value::as_str).unwrap_or("left") {
        "left" => false,
        "right" => true,
        _ => return Err("corner must be left or right".into()),
    };

    check((3.0..=50.0).contains(&t), "thickness must be 3 to 50 mm")?;
    check(
        (0.0..=20.0).contains(&shelves) && shelves.fract() == 0.0,
        "shelves must be 0 to 20",
    )?;
    check(plinth >= 0.0, "plinth cannot be negative")?;
    check(
        a <= 5000.0 && b <= 5000.0 && h <= 5000.0,
        "sizes are limited to 5000 mm",
    )?;
    check(d >= 150.0 + t, "depth is too small")?;
    check(
        a >= d + 200.0 && b >= d + 200.0,
        "width and return must each be 200 mm more than the depth",
    )?;
    let clear_h = h - plinth - 2.0 * t; // between bottom and top panels
    check(
        (clear_h - shelves * t) / (shelves + 1.0) >= 20.0,
        "height is too small for the plinth and shelves",
    )?;
    check(
        !doors || t >= 16.0,
        "doors with hinge cups need panels of 16 mm or more",
    )?;
    check(
        !doors || h - plinth >= 300.0,
        "too low for a door with two hinges",
    )?;

    let ea = kit.parameter("width", a, "along the back wall");
    let eb = kit.parameter("return", b, "along the side wall");
    let ed = kit.parameter("depth", d, "depth of both legs");
    let eh = kit.parameter("height", h, "outside height, plinth included");
    let et = kit.parameter("thickness", t, "panel thickness");
    let epl = match plinth > 0.0 {
        true => kit.parameter("plinth", plinth, "plinth height"),
        false => mm(0.0),
    };
    kit.centre(&ea, &eb);

    // Mirrored for a right corner: where a box of this length starts, and where a point is.
    let from = |x: &str, length: &str| match right {
        true => format!("{ea} - ({x}) - ({length})"),
        false => x.to_string(),
    };
    let point = |x: &str| from(x, "0 mm");
    let flip = if right { -1 } else { 1 };
    let panel = |part: &str, corner: [&str; 3], size: [&str; 3], cut: Option<&[String]>| {
        let x = from(corner[0], size[0]);
        kit.panel(part, [&x, corner[1], corner[2]], size, cut)
    };

    // The end of the back leg and the end of the return are the cabinet's two sides: together
    // they span its whole footprint, which is what placing against it measures.
    let (back_end, return_end) = match right {
        true => ("Side L", "Side R"),
        false => ("Side R", "Side L"),
    };
    let zero = mm(0.0);
    panel(
        back_end,
        [&format!("{ea} - {et}"), &format!("{eb} - {ed}"), &zero],
        [&et, &ed, &eh],
        None,
    );
    panel(return_end, [&zero, &zero, &zero], [&ed, &et, &eh], None);
    // Full panels against both walls, standing on the floor like the sides.
    panel(
        "Back",
        [&et, &format!("{eb} - {et}"), &zero],
        [&format!("{ea} - 2*{et}"), &et, &eh],
        None,
    );
    panel(
        "Back return",
        [&zero, &et, &zero],
        [&et, &format!("{eb} - {et}"), &eh],
        None,
    );
    // Bottom, top and shelves are L-shaped: a board over the whole inside, with the open
    // corner cut out of it. The cut list gives the board before the cut.
    let ech = format!("({eh} - {epl} - 2*{et})"); // clear height
    let s = shelves as u32;
    let egap = format!("(({ech} - {s}*{et}) / {})", s + 1); // between shelves
    let mut boards = vec![
        ("Bottom".to_string(), epl.clone()),
        ("Top".to_string(), format!("{eh} - {et}")),
    ];
    for i in 0..s {
        let z = format!("{epl} + {et} + {egap}*{} + {et}*{i}", i + 1);
        boards.push((format!("Shelf {}", i + 1), z));
    }
    for (part, z) in &boards {
        panel(
            part,
            [&et, &et, z],
            [&format!("{ea} - 2*{et}"), &format!("{eb} - 2*{et}"), &et],
            None,
        );
        panel(
            &format!("{part} notch"),
            [&ed, &et, z],
            [
                &format!("{ea} - {et} - {ed}"),
                &format!("{eb} - {ed} - {et}"),
                &et,
            ],
            Some(&[format!("{name} {part}")]),
        );
    }
    if plinth > 0.0 {
        // Two boards set back from the fronts, meeting in the inner corner.
        let x = format!("{ed} - {} - {et}", mm(KICK_RECESS));
        let y = format!("{eb} - {ed} + {}", mm(KICK_RECESS));
        panel(
            "Plinth",
            [&x, &y, &zero],
            [&format!("{ea} - {et} - ({x})"), &et, &epl],
            None,
        );
        panel(
            "Plinth return",
            [&x, &et, &zero],
            [&et, &format!("{y} - {et}"), &epl],
            None,
        );
    }

    if doors {
        // One overlay door on each front. They hinge on the two sides and stop short of the
        // inner corner, each leaving room for the other's thickness.
        let gap = mm(DOOR_GAP);
        // A door starts past the other door and its gaps, and ends a gap short of its side.
        let start = format!("{et} + {}", mm(2.0 * DOOR_GAP));
        let clear = format!("({et} + {})", mm(3.0 * DOOR_GAP));
        let (z0, z1) = (format!("{epl} + {gap}"), format!("{eh} - {gap}"));
        let door_h = format!("{eh} - {epl} - {}", mm(2.0 * DOOR_GAP));
        let mut cups = vec![
            format!("{z0} + {}", mm(CUP_FROM_END)),
            format!("{z1} - {}", mm(CUP_FROM_END)),
        ];
        if h - plinth - 2.0 * DOOR_GAP > 1000.0 {
            cups.push(format!("({z0} + {z1}) / 2"));
        }
        // (part, cup position x and y on the door's back face, and the way into the door)
        let cup = |part: &str, x: &str, y: &str, into: [i32; 3]| {
            for (j, z) in cups.iter().enumerate() {
                kit.push((
                    "solid.hole",
                    json!({
                        "position": kit.at(&point(x), y, z), "direction": into,
                        "diameter": mm(CUP_D), "depth": mm(CUP_DEPTH),
                        "targets": [format!("{name} {part}")],
                        "name": format!("{name} {part} hinge cup {}", j + 1),
                    }),
                ));
            }
        };
        let (g, floor) = (DOOR_GAP, plinth + DOOR_GAP);

        // On the back leg, facing the front: hinged on the back leg's side.
        let face_y = format!("{eb} - {ed} - {gap}"); // the door's back face
        kit.open("Door");
        panel(
            "Door",
            [&format!("{ed} + {start}"), &format!("{face_y} - {et}"), &z0],
            [&format!("{ea} - {ed} - {clear}"), &et, &door_h],
            None,
        );
        let cup_x = format!("{ea} - {gap} - {}", mm(CUP_CENTRE));
        cup("Door", &cup_x, &face_y, [0, -1, 0]);
        // like the box cabinet's: about the door's front edge, so it clears the next cabinet
        let hinge = [(a / 2.0 - g) * flip as f64, b / 2.0 - d - g - t, floor];
        kit.close("Door", "revolute", "hinge", hinge, [0, 0, flip], 110.0);

        // On the return, facing across: hinged on the return's side, at the front.
        let face_x = format!("{ed} + {gap}");
        kit.open("Door return");
        panel(
            "Door return",
            [&face_x, &gap, &z0],
            [&et, &format!("{eb} - {ed} - {clear}"), &door_h],
            None,
        );
        let cup_y = format!("{gap} + {}", mm(CUP_CENTRE));
        cup("Door return", &face_x, &cup_y, [flip, 0, 0]);
        let hinge = [(d + g + t - a / 2.0) * flip as f64, g - b / 2.0, floor];
        kit.close(
            "Door return",
            "revolute",
            "hinge",
            hinge,
            [0, 0, -flip],
            110.0,
        );
    }

    // The bounding box: `return` is its depth.
    Ok(kit.finish(Vec::new(), [a, h, b]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named<'a>(commands: &'a Commands, name: &str) -> &'a Value {
        let found = commands.iter().find(|c| c.1["name"] == name);
        &found.unwrap_or_else(|| panic!("no {name}")).1
    }

    #[test]
    fn an_l_of_boards_with_the_corner_cut_out() {
        let p = json!({"width": 1000, "return": 900, "plinth": 100, "doors": true});
        let (commands, summary) = build(&p).unwrap();
        // 2 sides, 2 backs, bottom, top, shelf, 2 plinth boards, 2 doors
        assert_eq!(summary["bodies"].as_array().unwrap().len(), 11);
        assert_eq!(
            summary["size"],
            json!({"width": 1000.0, "height": 720.0, "depth": 900.0})
        );
        assert_eq!(summary["joints"].as_array().unwrap().len(), 2);
        // the return's end is the left side, across the front of the footprint
        assert_eq!(
            named(&commands, "Cabinet Side L")["length"],
            "Cabinet_depth"
        );
        let notch = named(&commands, "Cabinet Shelf 1 notch");
        assert_eq!(notch["operation"], "cut");
        assert_eq!(notch["targets"], json!(["Cabinet Shelf 1"]));
        assert_eq!(notch["corner"][0], "Cabinet_depth - Cabinet_width/2");
    }

    #[test]
    fn a_right_corner_is_the_mirror_image() {
        let p = json!({"corner": "right", "doors": true});
        let (commands, _) = build(&p).unwrap();
        // the return's end is now the right side, and starts `depth` from the right
        assert_eq!(
            named(&commands, "Cabinet Side R")["corner"][0],
            "Cabinet_width - (0 mm) - (Cabinet_depth) - Cabinet_width/2"
        );
        let cup = named(&commands, "Cabinet Door return hinge cup 1");
        assert_eq!(cup["direction"], json!([-1, 0, 0]));
        let joint = commands
            .iter()
            .find(|c| c.1["name"] == "Cabinet Door hinge");
        // hinged at the left end of the back leg: 2 mm in from x = -450
        assert_eq!(joint.unwrap().1["at"]["point"][0], -448.0);
        assert_eq!(joint.unwrap().1["at"]["z"], json!([0, 0, -1]));
    }

    #[test]
    fn refuses_bad_input() {
        assert!(build(&json!({"width": 700})).is_err()); // no room beside the return
        assert!(build(&json!({"corner": "middle"})).is_err());
        assert!(build(&json!({"doors": true, "thickness": 12})).is_err());
        assert!(build(&json!({"height": 200, "shelves": 8})).is_err());
    }
}

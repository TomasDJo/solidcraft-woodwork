//! L-shaped corner cabinet: one leg along the back wall, the other (the return) running
//! forward along a side wall.
//!
//! Written for a corner at the back left: the footprint's bounding box is `width` (X) by
//! `return` (Y, the front at 0), and the return is on the left. A right corner is the same
//! mirrored in X. Like the box cabinet, every size is an expression over design parameters.

use crate::cabinet::{HANDLE_D, HANDLE_EDGE, HANDLE_END, PIN_D, PIN_SETBACK, PITCH, PULL};
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
    let pins = p
        .get("shelf_pins")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    // The bi-fold pair hangs on the end of the back leg (`width`) or of the return.
    let on_return = match p.get("doors").and_then(Value::as_str).unwrap_or("none") {
        "none" => None,
        "hinged on width" => Some(false),
        "hinged on return" => Some(true),
        _ => return Err("doors must be none, hinged on width or hinged on return".into()),
    };
    let doors = on_return.is_some();
    let handles = p.get("handles").and_then(Value::as_str).unwrap_or("none");
    check(
        matches!(handles, "none" | "knob" | "pull"),
        "handles must be none, knob or pull",
    )?;
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
    check(
        !pins || t >= 12.0,
        "shelf pin holes need panels of 12 mm or more",
    )?;
    check(
        !pins || clear_h >= 4.0 * PITCH,
        "too low for shelf pin holes",
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
    let elast = format!("floor(({ech} - {}) / {})", mm(2.0 * PITCH), mm(PITCH)); // top pin row
    for i in 0..s {
        // Height over the bottom panel: evenly spaced, or on the nearest row of pins.
        let mut over = format!("{egap}*{} + {et}*{i}", i + 1);
        if pins {
            let (pitch, r) = (mm(PITCH), mm(PIN_D / 2.0));
            over = format!("min(max(round(({over} - {r}) / {pitch}), 2), {elast}) * {pitch} + {r}");
        }
        boards.push((format!("Shelf {}", i + 1), format!("{epl} + {et} + {over}")));
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

    let mut drill = Vec::new(); // holes, run after the panels exist
    if pins {
        // Like the box cabinet's: one blind hole per panel, repeated up it and across to a
        // second row. Two rows in each end and in each back, 37 mm from the corners of a leg.
        let blind = format!("min({et} - {}, {})", mm(5.0), mm(12.0));
        let (near, far) = (
            format!("{et} + {}", mm(PIN_SETBACK)),
            format!("{eb} - {ed} + {}", mm(PIN_SETBACK)),
        );
        let (wall_x, wall_y) = (format!("{ea} - {et}"), format!("{eb} - {et}"));
        // (panel, the first hole on its inner face, the way in, the way to the second row)
        let rows = [
            (back_end, [&wall_x, &far], [flip, 0, 0], [0, 1, 0]),
            (return_end, [&near, &et], [0, -1, 0], [flip, 0, 0]),
            ("Back", [&near, &wall_y], [0, 1, 0], [flip, 0, 0]),
            ("Back return", [&et, &far], [-flip, 0, 0], [0, 1, 0]),
        ];
        for (part, [x, y], into, across) in rows {
            let feature = format!("{name} {part} pins");
            let z = format!("{epl} + {et} + {}", mm(2.0 * PITCH));
            drill.push((
                "solid.hole",
                json!({
                    "position": kit.at(&point(x), y, &z), "direction": into,
                    "diameter": mm(PIN_D), "depth": blind,
                    "targets": [format!("{name} {part}")], "name": feature,
                }),
            ));
            drill.push((
                "solid.pattern.rectangular",
                json!({
                    "features": [feature], "name": format!("{name} {part} pin rows"),
                    "dir1": [0, 0, 1], "count1": format!("{elast} - 1"), "spacing1": mm(PITCH),
                    "dir2": across, "count2": 2,
                    "spacing2": format!("{ed} - {et} - {}", mm(2.0 * PIN_SETBACK)),
                }),
            ));
        }
    }

    if let Some(on_return) = on_return {
        // A bi-fold pair of overlay doors, one on each front. The first hinges on the end of
        // its leg and runs into the inner corner, closing it; the second stops against the
        // first's face, hangs on it there and folds out flat with it.
        let gap = mm(DOOR_GAP);
        // Each door's end at the inner corner: the first's, and the second's a panel and a
        // gap short of it.
        let (long_x, short_x) = (
            format!("{ed} + {gap}"),
            format!("{ed} + {et} + {}", mm(2.0 * DOOR_GAP)),
        );
        let (long_y, short_y) = (
            format!("{eb} - {ed} - {gap}"),
            format!("{eb} - {ed} - {et} - {}", mm(2.0 * DOOR_GAP)),
        );
        let (z0, z1) = (format!("{epl} + {gap}"), format!("{eh} - {gap}"));
        let door_h = format!("{eh} - {epl} - {}", mm(2.0 * DOOR_GAP));
        let mut cups = vec![
            format!("{z0} + {}", mm(CUP_FROM_END)),
            format!("{z1} - {}", mm(CUP_FROM_END)),
        ];
        if h - plinth - 2.0 * DOOR_GAP > 1000.0 {
            cups.push(format!("({z0} + {z1}) / 2"));
        }
        // The first door carries the handle: upright on its opening edge, at the height the
        // box cabinet would put it.
        let half = if handles == "pull" { PULL / 2.0 } else { 0.0 };
        let end = mm(HANDLE_END + half);
        let handle_z = match (h - plinth > 1200.0, plinth > 0.0) {
            (true, _) => format!("({z0} + {z1}) / 2"),
            (_, true) => format!("{z1} - {end}"),
            _ => format!("{z0} + {end}"),
        };
        let screws: Vec<String> = match handles {
            "none" => vec![],
            "knob" => vec![handle_z],
            _ => vec![
                format!("{handle_z} - {}", mm(half)),
                format!("{handle_z} + {}", mm(half)),
            ],
        };
        // Holes from a door's back face: hinge cups at `cup`, and on the first door the handle
        // screws at `pull`. Both are x and y; `into` is the way into the door.
        let holes = |part: &str, cup: [&str; 2], pull: [&str; 2], first: bool, into: [i32; 3]| {
            let hole = |at: [&str; 2], z: &str, d: f64, depth: &str, what: String| {
                kit.push((
                    "solid.hole",
                    json!({
                        "position": kit.at(&point(at[0]), at[1], z), "direction": into,
                        "diameter": mm(d), "depth": depth,
                        "targets": [format!("{name} {part}")],
                        "name": format!("{name} {part} {what}"),
                    }),
                ));
            };
            for (j, z) in cups.iter().enumerate() {
                hole(
                    cup,
                    z,
                    CUP_D,
                    &mm(CUP_DEPTH),
                    format!("hinge cup {}", j + 1),
                );
            }
            for (j, z) in screws.iter().enumerate().filter(|_| first) {
                hole(pull, z, HANDLE_D, &et, format!("handle {}", j + 1));
            }
        };
        let (g, floor) = (DOOR_GAP, plinth + DOOR_GAP);
        let (cup_c, edge) = (mm(CUP_CENTRE), mm(HANDLE_EDGE));
        let face_y = &long_y; // the back leg door's back face
                              // On the back leg, facing the front.
        let door_x = if on_return { &short_x } else { &long_x };
        let door = || {
            kit.open("Door");
            panel(
                "Door",
                [door_x, &format!("{face_y} - {et}"), &z0],
                [&format!("{ea} - {gap} - ({door_x})"), &et, &door_h],
                None,
            );
            let cup_x = match on_return {
                true => format!("{short_x} + {cup_c}"),
                false => format!("{ea} - {gap} - {cup_c}"),
            };
            let pull_x = format!("{short_x} + {edge}");
            holes(
                "Door",
                [&cup_x, face_y],
                [&pull_x, face_y],
                !on_return,
                [0, -1, 0],
            );
        };
        // On the return, facing across.
        let face_x = &long_x; // its back face
        let return_y = if on_return { &long_y } else { &short_y };
        let door_return = || {
            kit.open("Door return");
            panel(
                "Door return",
                [face_x, &gap, &z0],
                [&et, &format!("{return_y} - {gap}"), &door_h],
                None,
            );
            let cup_y = match on_return {
                true => format!("{gap} + {cup_c}"),
                false => format!("{short_y} - {cup_c}"),
            };
            let pull_y = format!("{short_y} - {edge}");
            holes(
                "Door return",
                [face_x, &cup_y],
                [face_x, &pull_y],
                on_return,
                [flip, 0, 0],
            );
        };
        // A hinge is about the door's front edge at the end of its leg, like the box cabinet's,
        // so the door clears the next cabinet. The fold is where the two back faces meet, as
        // on a bi-fold hinge: 90 degrees brings the second door in line with the first.
        let (front_y, meet_x) = (b / 2.0 - d - g - t, (d + g + t - a / 2.0) * flip as f64);
        let fold = [meet_x - t * flip as f64, front_y + t, floor];
        let close = |part: &str, on: Option<&str>, joint: &str, at: [f64; 3], turn: i32| {
            let limit = if on.is_some() { 90.0 } else { 110.0 };
            kit.close(part, on, "revolute", joint, at, [0, 0, turn], limit)
        };
        if on_return {
            door_return();
            let hinge = [meet_x, g - b / 2.0, floor];
            close("Door return", None, "hinge", hinge, -flip);
            door();
            close("Door", Some("Door return"), "fold", fold, flip);
        } else {
            door();
            let hinge = [(a / 2.0 - g) * flip as f64, front_y, floor];
            close("Door", None, "hinge", hinge, flip);
            door_return();
            close("Door return", Some("Door"), "fold", fold, -flip);
        }
    }

    // The bounding box: `return` is its depth.
    Ok(kit.finish(drill, [a, h, b]))
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
        let p = json!({"width": 1000, "return": 900, "plinth": 100, "doors": "hinged on width"});
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
        let p = json!({"corner": "right", "doors": "hinged on width"});
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
        assert!(build(&json!({"doors": "hinged on width", "thickness": 12})).is_err());
        assert!(build(&json!({"height": 200, "shelves": 8})).is_err());
    }
}

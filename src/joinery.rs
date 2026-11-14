//! Screw or dowel holes where the sides of a cabinet meet its bottom and top.
//!
//! The holes are expressions over the cabinet's parameters, like the cabinet itself, and are
//! drilled inside its component, so they follow a resize and a move.

use crate::{cabinet, select, Commands};
use serde_json::{json, Value};

const FROM_EDGE: f64 = 50.0; // first and last hole, from the front and back edge
const MAX_SPACING: f64 = 250.0;

/// `design` is `document.inspect`'s reply with the timeline, `kind` is confirmat or dowel.
/// The commands run with the cabinet's component active. Also returns where the cabinet's
/// origin (the middle of its footprint, on its floor) is in the world, in mm.
pub fn build(
    found: &select::Cabinet,
    design: &Value,
    kind: &str,
) -> Result<(Commands, [f64; 3]), String> {
    let name = &found.name;
    let prefix = cabinet::prefix(name);
    let value = |key: &str| {
        let id = format!("{prefix}_{key}");
        let all = design["params"].as_array().into_iter().flatten();
        all.into_iter()
            .find(|p| p["name"] == id.as_str())
            .and_then(|p| p["value"].as_f64())
    };
    let (Some(depth), Some(t)) = (value("depth"), value("thickness")) else {
        return Err(format!(
            "`{name}` has no parameters: joinery needs a cabinet made by version 0.0.7 or later"
        ));
    };
    let label = format!("{name} joint");
    let features = design["timeline"].as_array().into_iter().flatten();
    if features
        .into_iter()
        .any(|f| f["name"].as_str().is_some_and(|n| n.starts_with(&label)))
    {
        return Err(format!("`{name}` already has joinery"));
    }

    let (ew, eh, ed, et) = (
        format!("{prefix}_width"),
        format!("{prefix}_height"),
        format!("{prefix}_depth"),
        format!("{prefix}_thickness"),
    );
    let epl = if value("plinth").is_some() {
        format!("{prefix}_plinth")
    } else {
        "0 mm".into()
    };
    // With an overlaid back the sides, bottom and top stop short of the full depth.
    let overlay = found.max[1] - found.min[1] < depth - 0.01;
    let side_d = if overlay {
        format!("({ed} - {prefix}_back_thickness)")
    } else {
        ed.clone()
    };
    // Two holes at least, and more on a deep cabinet, evenly between the first and the last.
    // ponytail: every hole is a feature of its own, so their number is settled here and only
    // the spacing follows a resize. (A pattern would follow, but the app loses copies of a
    // patterned hole once another cabinet stands where this one's numbers point.)
    let run = found.max[1] - found.min[1] - 2.0 * FROM_EDGE;
    let count = ((run / MAX_SPACING).ceil() as u32 + 1).max(2);
    let spacing = format!("({side_d} - {} mm) / {}", 2.0 * FROM_EDGE, count - 1);
    // Hole in the side, then hole in the end of the bottom or top: (name, diameter, depth).
    let (in_side, in_end) = match kind {
        // 7 x 50 mm confirmat: through the side from outside, pilot in the panel's end.
        "confirmat" => {
            if t >= 40.0 {
                return Err(
                    "panels of 40 mm or more are too thick for 50 mm confirmat screws".into(),
                );
            }
            (
                ("clearance", "7 mm", et.clone()),
                ("pilot", "5 mm", format!("50 mm - {et}")),
            )
        }
        // 8 x 35 mm dowel: blind in the side's inner face, the rest in the panel's end.
        "dowel" => (
            ("dowel side", "8 mm", format!("min({et} - 5 mm, 13 mm)")),
            ("dowel end", "8 mm", "22 mm".into()),
        ),
        _ => return Err("type must be confirmat or dowel".into()),
    };

    // Positions from the cabinet's front left bottom corner; the cabinet is centred on its origin.
    let place = |x: &str, k: u32, z: &str| {
        json!([
            format!("{x} - {ew}/2"),
            format!("{FROM_EDGE} mm + {spacing} * {k} - {ed}/2"),
            z
        ])
    };
    let mut commands = Vec::new();
    let sides = [
        ("Side L", "0 mm".to_string(), et.clone(), 1),
        ("Side R", ew.clone(), format!("{ew} - {et}"), -1),
    ];
    // `inwards` is the X direction from the side into the cabinet.
    for (side, outer, inner, inwards) in &sides {
        let panels = [
            ("Bottom", format!("{epl} + {et}/2")),
            ("Top", format!("{eh} - {et}/2")),
        ];
        for (panel, z) in &panels {
            // A screw goes in from the outer face, a dowel sits behind the inner face.
            let (from, into_side) = match kind {
                "confirmat" => (outer, *inwards),
                _ => (inner, -inwards),
            };
            let holes = [
                (in_side.clone(), from, side, into_side),
                (in_end.clone(), inner, panel, *inwards),
            ];
            for ((what, diameter, depth), x, target, direction) in holes {
                for k in 0..count {
                    // the first is named plainly: the cut list finds the joinery by it
                    let feature = match k {
                        0 => format!("{label} {side} {panel} {what}"),
                        _ => format!("{label} {side} {panel} {what} {}", k + 1),
                    };
                    commands.push((
                        "solid.hole",
                        json!({
                            "position": place(x, k, z), "direction": [direction, 0, 0],
                            "diameter": diameter, "depth": depth,
                            "targets": [format!("{name} {target}")], "name": feature,
                        }),
                    ));
                }
            }
        }
    }
    let features: Vec<&Value> = commands.iter().map(|c| &c.1["name"]).collect();
    let group = json!({ "name": format!("{name} joinery"), "features": features });
    commands.push(("timeline.group", group));
    let origin = [
        (found.min[0] + found.max[0]) / 2.0,
        found.min[1] + depth / 2.0,
        found.min[2],
    ];
    Ok((commands, origin))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found() -> select::Cabinet {
        select::Cabinet {
            name: "Base 1".into(),
            min: [0.0, 0.0, 0.0],
            max: [600.0, 554.0, 720.0],
        }
    }

    #[test]
    fn four_joints_of_two_holes_and_a_row() {
        let design = json!({"timeline": [], "params": [
            {"name": "Base_1_depth", "value": 560.0}, {"name": "Base_1_thickness", "value": 18.0},
            {"name": "Base_1_plinth", "value": 100.0},
        ]});
        let (commands, origin) = build(&found(), &design, "confirmat").unwrap();
        assert_eq!(origin, [300.0, 280.0, 0.0]);
        // sides 554 deep in a 560 cabinet: 3 screws per joint, and an overlaid back
        assert_eq!(commands.len(), 4 * 2 * 3 + 1);
        assert_eq!(
            commands[2].1["position"][1],
            "50 mm + ((Base_1_depth - Base_1_back_thickness) - 100 mm) / 2 * 2 - Base_1_depth/2"
        );
        // the screw enters the left side from outside, the bottom sits on the plinth
        assert_eq!(commands[0].1["position"][0], "0 mm - Base_1_width/2");
        assert_eq!(
            commands[0].1["position"][2],
            "Base_1_plinth + Base_1_thickness/2"
        );
        assert_eq!(commands[3].1["targets"][0], "Base 1 Bottom");

        let (dowels, _) = build(&found(), &design, "dowel").unwrap();
        assert_eq!(
            dowels[0].1["position"][0],
            "Base_1_thickness - Base_1_width/2"
        );
    }

    #[test]
    fn refuses_twice_and_unknown_cabinets() {
        let params = json!([{"name": "Base_1_depth", "value": 560.0}, {"name": "Base_1_thickness", "value": 18.0}]);
        let drilled =
            json!({"timeline": [{"name": "Base 1 joint Side L Bottom pilot"}], "params": params});
        assert!(build(&found(), &drilled, "dowel").is_err());
        assert!(build(&found(), &json!({"timeline": [], "params": []}), "dowel").is_err());
        assert!(build(&found(), &json!({"timeline": [], "params": params}), "glue").is_err());
    }
}

//! Screw or dowel holes where the sides of a cabinet meet its bottom and top.
//!
//! The holes are expressions over the cabinet's parameters, like the cabinet itself, and are
//! drilled inside its component, so they follow a resize and a move.

use crate::{cabinet, select, Commands};
use serde_json::{json, Value};

const FROM_EDGE: &str = "50 mm"; // first and last hole, from the front and back edge
const MAX_SPACING: &str = "250 mm";

/// `design` is `document.inspect`'s reply with the timeline, `kind` is confirmat or dowel.
/// The commands run with the cabinet's component active.
pub fn build(found: &select::Cabinet, design: &Value, kind: &str) -> Result<Commands, String> {
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
    let count = format!("max(2, ceil(({side_d} - 2*{FROM_EDGE}) / {MAX_SPACING}) + 1)");
    let spacing = format!("({side_d} - 2*{FROM_EDGE}) / ({count} - 1)");
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
    let place =
        |x: &str, z: &str| json!([format!("{x} - {ew}/2"), format!("{FROM_EDGE} - {ed}/2"), z]);
    let mut commands = Vec::new();
    let sides = [
        ("Side L", "0 mm".to_string(), et.clone()),
        ("Side R", ew.clone(), format!("{ew} - {et}")),
    ];
    for (side, outer, inner) in &sides {
        let panels = [
            ("Bottom", format!("{epl} + {et}/2")),
            ("Top", format!("{eh} - {et}/2")),
        ];
        for (panel, z) in &panels {
            // A screw goes in from the outer face, a dowel sits behind the inner face.
            let from = if kind == "confirmat" { outer } else { inner };
            let holes = [
                (in_side.clone(), from, side),
                (in_end.clone(), inner, panel),
            ];
            let mut row = Vec::new();
            for ((what, diameter, depth), x, target) in holes {
                let feature = format!("{label} {side} {panel} {what}");
                commands.push((
                    "solid.hole",
                    json!({
                        "position": place(x, z), "diameter": diameter, "depth": depth,
                        "targets": [format!("{name} {target}")], "name": feature,
                    }),
                ));
                row.push(feature);
            }
            commands.push((
                "solid.pattern.rectangular",
                json!({
                    "features": row, "name": format!("{label} {side} {panel} row"),
                    "dir1": [0, 1, 0], "count1": count, "spacing1": spacing,
                }),
            ));
        }
    }
    let features: Vec<&Value> = commands.iter().map(|c| &c.1["name"]).collect();
    let group = json!({ "name": format!("{name} joinery"), "features": features });
    commands.push(("timeline.group", group));
    Ok(commands)
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
        let commands = build(&found(), &design, "confirmat").unwrap();
        assert_eq!(commands.len(), 4 * 3 + 1);
        // sides 554 deep in a 560 cabinet: an overlaid back
        assert!(commands[2].1["count1"]
            .as_str()
            .unwrap()
            .contains("Base_1_depth - Base_1_back_thickness"));
        // the screw enters the left side from outside, the bottom sits on the plinth
        assert_eq!(commands[0].1["position"][0], "0 mm - Base_1_width/2");
        assert_eq!(
            commands[0].1["position"][2],
            "Base_1_plinth + Base_1_thickness/2"
        );
        assert_eq!(commands[1].1["targets"][0], "Base 1 Bottom");

        let dowels = build(&found(), &design, "dowel").unwrap();
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

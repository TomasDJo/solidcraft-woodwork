//! Woodworking plugin for SolidCraft. The modules are plain Rust that turn parameters into
//! app commands, so `cargo test` covers them; only `guest` touches the wasm bindings.

mod cabinet;
mod corner;
mod cutlist;
mod edit;
mod fittings;
mod joinery;
mod kit;
mod material;
mod select;

use kit::check;
use serde_json::{json, Value};

/// App commands with their arguments, in the order they run.
pub type Commands = Vec<(&'static str, Value)>;

/// Runs one app command and returns its result.
pub type Run<'a> = &'a mut dyn FnMut(&str, Value) -> Result<Value, String>;

/// Builds a cabinet as a component of its own inside whichever component is active, so that it
/// moves as one piece and a room can hold its cabinets. Gives its boards their material, and
/// leaves on the component how it was made, for Edit. Returns the summary and
/// `component.create`'s reply.
fn make(entry: &str, params: &Value, run: Run) -> Result<(Value, Value), String> {
    let (commands, mut summary) = match entry {
        "cabinet" => cabinet::build(params)?,
        _ => corner::build(params)?,
    };
    let made = run(
        "component.create",
        json!({"name": summary["name"], "grounded": false}),
    )?;
    summary["occurrence"] = made["occurrence"].clone();
    for (command, args) in commands {
        run(command, args)?;
    }
    let board = params["material"].as_str().unwrap_or("Birch plywood");
    if board != "none" {
        let bodies = summary["bodies"].as_array().into_iter().flatten();
        let bodies: Vec<String> = bodies
            .filter_map(|b| b.as_str())
            .map(String::from)
            .collect();
        material::assign(run, board, &bodies)?;
    }
    let name = summary["name"].as_str().unwrap_or_default();
    run(
        "component.metadata.set",
        json!({"component": name, "values": edit::remember(entry, params, name)}),
    )?;
    // Back to where the user was working.
    run("component.activate", json!({"component": made["parent"]}))?;
    Ok((summary, made))
}

/// Holds a cabinet that was placed against another to that one, with a rigid joint whose
/// distance is an expression over the two cabinets' parameters: when either changes size the
/// placed one keeps standing against its neighbour, and what is placed against it in turn
/// follows. `at` is where the placed cabinet's origin is now. Notes the link on its component,
/// so Edit can make it again. False when the other cabinet has no such parameter.
// ponytail: only the distance follows. A cabinet that is made deeper than its neighbour
// afterwards keeps its middle where it was, so the backs are no longer flush: the joint's
// sideways offsets are worked out once. A second joint cannot hold the same two parts.
fn link(
    other: &select::Cabinet,
    place: &str,
    name: &str,
    at: [f64; 3],
    run: Run,
) -> Result<bool, String> {
    let design = run(
        "document.inspect",
        json!({"bodies": false, "timeline": false}),
    )?;
    let value = |id: &str| {
        let all = design["params"].as_array().into_iter().flatten();
        let found = all.into_iter().find(|p| p["name"] == id);
        found.and_then(|p| p["value"].as_f64())
    };
    let (toward, size) = other.toward(place);
    let (theirs, ours) = (
        format!("{}_{size}", kit::prefix(&other.name)),
        format!("{}_width", kit::prefix(name)),
    );
    let (offset, gap) = match (place, value(&theirs), value(&ours)) {
        ("above", Some(height), _) => (theirs, height),
        (_, Some(a), Some(b)) if place != "above" => {
            (format!("{theirs}/2 + {ours}/2"), (a + b) / 2.0)
        }
        _ => return Ok(false),
    };
    // Both frames are given where the cabinets stand: the placed one's at its origin, the
    // other's that distance back, with the same directions, so nothing moves now.
    let from: Vec<f64> = (0..3).map(|i| at[i] - toward[i] * gap).collect();
    let x = if place == "above" {
        [1, 0, 0]
    } else {
        [0, 0, 1]
    };
    let frame = |cabinet: &str, origin: &[f64]| {
        json!({
            "occurrence": format!("{cabinet}:1"),
            "frame": {"origin": origin, "z": toward, "x": x},
        })
    };
    run(
        "joint.create",
        json!({
            "type": "rigid", "name": format!("{name} link"), "flip": true, "offset": offset,
            "a": frame(&other.name, &from), "b": frame(name, &at), "quiet": true,
        }),
    )?;
    let about = run("component.metadata", json!({"component": name}))?;
    let mut ours = about["metadata"][edit::KEY].clone();
    if ours.is_object() {
        ours["link"] = json!({"to": other.name, "place": place});
        run(
            "component.metadata.set",
            json!({"component": name, "values": { edit::KEY: ours }}),
        )?;
    }
    Ok(true)
}

/// Drills a cabinet's screw or dowel holes inside its component, and notes the kind on the
/// component so that Edit can drill them again.
fn drill(found: &select::Cabinet, kind: &str, run: Run) -> Result<(), String> {
    let design = run(
        "document.inspect",
        json!({"bodies": false, "timeline": "summary"}),
    )?;
    let (commands, origin) = joinery::build(found, &design, kind)?;
    let tree = run("component.list", json!({"compact": true}))?;
    let before = tree["active"].clone();
    // In a plugin run the app shifts a hole's position by where the component stands,
    // so a cabinet that has been moved is drilled on the origin and then put back.
    // ponytail: this leaves a cabinet that is not held to another grounded, as the cabinet
    // entry made it, also if the user had freed it since.
    let occurrence = format!("{}:1", found.name);
    let shift = |by: [f64; 3], run: Run| {
        let moved = run(
            "occurrence.move",
            json!({"occurrence": occurrence, "translate": by}),
        )?;
        // a joint to something else puts it straight back, and the holes would miss
        check(
            moved["reverted"] != true,
            "the cabinet is held by a joint and cannot be moved to drill it",
        )
    };
    let away = origin.iter().any(|v| v.abs() > 1e-6);
    // Any link holds the cabinet where it is, its own to the cabinet it stands against and
    // those of the cabinets that stand against it: all are put aside for the move. Without
    // one of its own it is grounded instead.
    let links = edit::links(&tree, &run("joint.list", json!({}))?, &found.name);
    let held = links.iter().any(|link| link.1);
    let free = |free: bool, run: Run| -> Result<(), String> {
        for (link, _) in &links {
            run("joint.edit", json!({"joint": link, "suppressed": free}))?;
        }
        if !held {
            run(
                "occurrence.ground",
                json!({"occurrence": occurrence, "grounded": !free}),
            )?;
        }
        Ok(())
    };
    if away {
        free(true, run)?;
        shift(origin.map(|v| -v), run)?;
    }
    run("component.activate", json!({"component": found.name}))?;
    for (command, args) in commands {
        run(command, args)?;
    }
    run("component.activate", json!({"component": before}))?;
    if away {
        shift(origin, run)?;
        free(false, run)?;
    }
    let about = run("component.metadata", json!({"component": found.name}))?;
    let mut ours = about["metadata"][edit::KEY].clone();
    if ours.is_object() {
        ours["joinery"] = json!(kind);
        run(
            "component.metadata.set",
            json!({"component": found.name, "values": { edit::KEY: ours }}),
        )?;
    }
    Ok(())
}

/// Writes a file into a folder the user approved at install: folder grant, file name, bytes.
pub type Write<'a> = &'a mut dyn FnMut(&str, &str, &[u8]) -> Result<(), String>;

/// Saves the cut list in `result` as a CSV file when its `file` field asks for one, and notes
/// the name in the result.
pub fn save(entry: &str, params: &Value, result: &mut Value, write: Write) -> Result<(), String> {
    if let Some(name) = cutlist::file_name(entry, params)? {
        let csv = result["csv"].as_str().unwrap_or_default();
        write(cutlist::FOLDER, &name, csv.as_bytes()).map_err(|reason| {
            format!(
                "could not save {name}: {reason}. Saving needs a folder for cut lists, approved when the plugin is installed: install it again and enter a folder, or clear Save as"
            )
        })?;
        result["saved"] = json!(name);
    }
    Ok(())
}

pub fn execute(entry: &str, params: &Value, run: Run) -> Result<Value, String> {
    match entry {
        "cabinet" | "corner" => {
            // Where it goes: on the origin, or against the cabinet that is selected.
            let mut place = params["place"].as_str().unwrap_or("origin");
            check(
                matches!(place, "origin" | "auto" | "right" | "left" | "above"),
                "place must be origin, auto, right, left or above",
            )?;
            let matching = match params["match"].as_str().unwrap_or("no") {
                "no" => false,
                "selected" => true,
                _ => return Err("match must be no or selected".into()),
            };
            let mut params = params.clone();
            let design = run(
                "document.inspect",
                json!({"bodies": false, "timeline": false}),
            )?;
            let picked = select::picked(&params, &design);
            let measure = match matching || place != "origin" {
                true => run("inspect.measure", json!({}))?,
                false => Value::Null,
            };
            if place == "auto" {
                // By the board that is selected: its side, or its top.
                place = select::side(&picked, &measure)?;
                params["place"] = json!(place);
            }
            if entry == "corner" {
                // Settled here, so the cabinet remembers which it became: Edit builds it
                // again without knowing where it was placed.
                params["corner"] = json!(corner::hand(&params)?);
            }
            let mut beside = None;
            // A name that is taken gets the next free number: Cabinet, Cabinet 2, Cabinet 3.
            let name = params["name"].as_str().unwrap_or("Cabinet");
            params["name"] = json!(kit::free_name(name, &design));
            if matching || place != "origin" {
                let found = select::cabinet(&picked, &measure)?;
                if matching {
                    params["_match"] = kit::matched(&found.name, &design)?;
                }
                if place != "origin" {
                    beside = Some(found);
                }
            }
            let (mut summary, made) = make(entry, &params, run)?;
            let mut held = false;
            if let Some(other) = beside {
                // Against that cabinet's side, or on top of it, turned the way it is turned.
                let size = |key: &str| summary["size"][key].as_f64().unwrap_or(0.0);
                let (at, degrees) = other.seat(place, size("width"), size("depth"));
                // plain numbers: this command takes mm whatever the design's unit is
                let mut to =
                    json!({"occurrence": made["occurrence"], "space": "world", "translate": at});
                if degrees != 0 {
                    to["axis"] = json!([0, 0, 1]);
                    to["angle"] = json!(format!("{degrees} deg"));
                }
                run("occurrence.move", to)?;
                summary["placed"] = json!(format!("{place} of {}", other.name));
                let name = summary["name"].as_str().unwrap_or_default().to_string();
                held = link(&other, place, &name, at, run)?;
                summary["linked"] = json!(held);
            }
            // Grounded, so that its doors and drawers have something fixed to open against,
            // unless it is held to the cabinet it stands against, which is. To move the
            // cabinet later, unground it first, or delete its link.
            if !held {
                run(
                    "occurrence.ground",
                    json!({"occurrence": made["occurrence"], "grounded": true}),
                )?;
            }
            Ok(summary)
        }
        "edit" => {
            // The selected cabinet, built again where it stands with some of its values
            // changed. Its parameters stay, so what is linked to them still follows.
            let design = run(
                "document.inspect",
                json!({"bodies": false, "timeline": false}),
            )?;
            let measure = run("inspect.measure", json!({}))?;
            let name = select::cabinet(&select::picked(params, &design), &measure)?.name;
            let about = run("component.metadata", json!({"component": name}))?;
            let (entry, params) = edit::merge(&about["metadata"][edit::KEY], params, &design)
                .map_err(|e| format!("`{name}`: {e}"))?;
            // Checked before anything is taken away.
            match entry.as_str() {
                "cabinet" => cabinet::build(&params)?,
                _ => corner::build(&params)?,
            };
            let tree = run("component.list", json!({}))?;
            let (parent, matrix, grounded) = edit::placement(&tree, &name)?;
            let materials = edit::materials(&measure, &select::parts(&measure, &name));
            // The cabinets held to this one lose their link with it: found now, held again after.
            let joints = run("joint.list", json!({}))?;
            let hanging = edit::hanging(&tree, &joints, &name);
            run("component.activate", json!({"component": parent}))?;
            let gone = run("component.delete", json!({"component": name}))?;
            let (mut summary, made) = make(&entry, &params, run)?;
            // Boards that had another material than the rest get it back.
            for (material, bodies) in materials {
                let still: Vec<&String> = (bodies.iter())
                    .filter(|b| {
                        summary["bodies"]
                            .as_array()
                            .is_some_and(|all| all.contains(&json!(b)))
                    })
                    .collect();
                if !still.is_empty() {
                    run(
                        "material.assign",
                        json!({"bodies": still, "material": material}),
                    )?;
                }
            }
            // Its screw or dowel holes went with the old boards: drilled again, before it is
            // put back where it stood.
            if let Some(kind) = about["metadata"][edit::KEY]["joinery"].as_str() {
                let found = select::named(&run("inspect.measure", json!({}))?, &name)?;
                drill(&found, kind, run)?;
                summary["joinery"] = json!(kind);
            }
            run(
                "occurrence.move",
                json!({"occurrence": made["occurrence"], "to_matrix": matrix, "move_grounded": true}),
            )?;
            // Held again to the cabinet it stood against, when that is still there; and the
            // cabinets that were held to it, to the new one.
            let stands = |matrix: &Value| [0, 1, 2].map(|i| matrix[i][3].as_f64().unwrap_or(0.0));
            let was = &about["metadata"][edit::KEY]["link"];
            let measure = run("inspect.measure", json!({}))?;
            let mut held = false;
            if let (Some(to), Some(place)) = (was["to"].as_str(), was["place"].as_str()) {
                if let Ok(other) = select::named(&measure, to) {
                    held = link(&other, place, &name, stands(&matrix), run)?;
                }
            }
            if !held {
                run(
                    "occurrence.ground",
                    json!({"occurrence": made["occurrence"], "grounded": grounded}),
                )?;
            }
            let this = select::named(&measure, &name)?;
            for other in hanging {
                let about = run("component.metadata", json!({"component": other}))?;
                let place = about["metadata"][edit::KEY]["link"]["place"].as_str();
                let (_, matrix, _) = edit::placement(&tree, &other)?;
                let held = match place {
                    Some(place) => link(&this, place, &other, stands(&matrix), run)?,
                    None => false,
                };
                if !held {
                    run(
                        "occurrence.ground",
                        json!({"occurrence": format!("{other}:1"), "grounded": true}),
                    )?;
                }
            }
            // Back to where the user was working, unless that was inside the old cabinet.
            let inside = gone["deleted"]
                .as_array()
                .is_some_and(|ids| ids.contains(&tree["active"]));
            if !inside {
                run("component.activate", json!({"component": tree["active"]}))?;
            }
            summary["edited"] = json!(true);
            Ok(summary)
        }
        "joinery" => {
            // Screw or dowel holes at the corners of the selected cabinet, drilled inside its
            // component.
            let design = run(
                "document.inspect",
                json!({"bodies": false, "timeline": false}),
            )?;
            let picked = select::picked(params, &design);
            let found = select::cabinet(&picked, &run("inspect.measure", json!({}))?)?;
            let kind = params["type"].as_str().unwrap_or("confirmat");
            drill(&found, kind, run)?;
            Ok(json!({"cabinet": found.name, "type": kind, "joints": 4}))
        }
        "material" => {
            // A board material for the selected cabinet, or for just the selected parts.
            let design = run(
                "document.inspect",
                json!({"bodies": false, "timeline": false}),
            )?;
            let picked = select::picked(params, &design);
            let bodies = match params["scope"].as_str().unwrap_or("cabinet") {
                "parts" => picked,
                "cabinet" => {
                    let measure = run("inspect.measure", json!({}))?;
                    select::parts(&measure, &select::cabinet(&picked, &measure)?.name)
                }
                _ => return Err("scope must be cabinet or parts".into()),
            };
            if bodies.is_empty() {
                return Err("select a cabinet or some of its parts first".into());
            }
            let board = params["material"].as_str().unwrap_or("Birch plywood");
            material::assign(run, board, &bodies)?;
            Ok(json!({"material": board, "bodies": bodies}))
        }
        "cutlist" => {
            // Inches when the design is in inches or feet, else mm.
            let design = run(
                "document.inspect",
                json!({"bodies": false, "timeline": "summary"}),
            )?;
            let imperial = matches!(design["units"].as_str(), Some("in" | "ft"));
            let measure = run("inspect.measure", json!({}))?;
            // ponytail: one more command per turned board, so a design with more than about
            // 250 boards standing at an angle runs into the 256-command limit.
            let mut local = std::collections::BTreeMap::new();
            for body in cutlist::turned(&measure) {
                let mesh = run("model.mesh", json!({"body": body, "tolerance": 1.0}))?;
                local.insert(body, cutlist::extents(&mesh));
            }
            let mut list = cutlist::cutlist(&measure, &local, imperial)?;
            list["hardware"] = cutlist::hardware(&measure, &design);
            Ok(list)
        }
        _ => Err(format!("unknown entry `{entry}`")),
    }
}

#[cfg(target_arch = "wasm32")]
mod guest {
    wit_bindgen::generate!({ path: "wit", world: "command-plugin" });

    use craft::plugin::types::{FieldFailure, PluginError};

    struct Woodwork;

    impl exports::craft::plugin::entry::Guest for Woodwork {
        fn execute(entry: String, params: String) -> Result<String, PluginError> {
            let invalid = |reason: String| {
                PluginError::InvalidInput(FieldFailure {
                    field: "params".into(),
                    reason,
                })
            };
            let params = serde_json::from_str(&params).map_err(|e| invalid(e.to_string()))?;
            let mut run = |command: &str, args: serde_json::Value| {
                let reply = craft::plugin::commands::run(command, &args.to_string()).map_err(
                    |e| match e {
                        PluginError::CommandFailed(f) => format!("{}: {}", f.command, f.reason),
                        other => format!("{command}: {other:?}"),
                    },
                )?;
                serde_json::from_str(&reply).map_err(|e| format!("{command}: bad reply: {e}"))
            };
            let mut result = crate::execute(&entry, &params, &mut run).map_err(invalid)?;
            let mut write = |folder: &str, name: &str, data: &[u8]| {
                craft::plugin::files::write_file(folder, name, data).map_err(|e| format!("{e:?}"))
            };
            crate::save(&entry, &params, &mut result, &mut write).map_err(invalid)?;
            Ok(result.to_string())
        }
    }

    export!(Woodwork);
}

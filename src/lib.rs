//! Woodworking plugin for SolidCraft. The modules are plain Rust that turn parameters into
//! app commands, so `cargo test` covers them; only `guest` touches the wasm bindings.

mod cabinet;
mod cutlist;
mod joinery;
mod material;
mod select;

use serde_json::{json, Value};

/// App commands with their arguments, in the order they run.
pub type Commands = Vec<(&'static str, Value)>;

/// Runs one app command and returns its result.
pub type Run<'a> = &'a mut dyn FnMut(&str, Value) -> Result<Value, String>;

pub fn execute(entry: &str, params: &Value, run: Run) -> Result<Value, String> {
    match entry {
        "cabinet" => {
            let (commands, mut summary) = cabinet::build(params)?;
            // Where it goes: on the origin, or against the cabinet that is selected.
            let place = params["place"].as_str().unwrap_or("origin");
            let beside = match place {
                "origin" => None,
                "right" | "left" | "above" => {
                    let design = run(
                        "document.inspect",
                        json!({"bodies": false, "timeline": false}),
                    )?;
                    Some(select::cabinet(
                        &design,
                        &run("inspect.measure", json!({}))?,
                    )?)
                }
                _ => return Err("place must be origin, right, left or above".into()),
            };
            // The cabinet is a component of its own, inside whichever component is active:
            // it moves as one piece, and a room can hold its cabinets.
            let made = run(
                "component.create",
                json!({"name": summary["name"], "grounded": false}),
            )?;
            summary["occurrence"] = made["occurrence"].clone();
            for (command, args) in commands {
                run(command, args).map_err(|e| match command {
                    // The parameters are named after the cabinet, so a second cabinet of
                    // the same name stops here, before anything is built.
                    "parameters.add" => format!(
                        "{e}. Is there already a cabinet of this name? Give this one another name."
                    ),
                    _ => e,
                })?;
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
            // Back to where the user was working.
            run("component.activate", json!({"component": made["parent"]}))?;
            if let Some(other) = beside {
                // Backs flush; side by side on the same floor, or on top with left sides flush.
                let size = |key: &str| summary["size"][key].as_f64().unwrap_or(0.0);
                let x = match place {
                    "right" => other.max[0] + size("width") / 2.0,
                    "left" => other.min[0] - size("width") / 2.0,
                    _ => other.min[0] + size("width") / 2.0,
                };
                let z = if place == "above" {
                    other.max[2]
                } else {
                    other.min[2]
                };
                // plain numbers: this command takes mm whatever the design's unit is
                let to = [x, other.max[1] - size("depth") / 2.0, z];
                run(
                    "occurrence.move",
                    json!({"occurrence": made["occurrence"], "space": "world", "translate": to}),
                )?;
                summary["placed"] = json!(format!("{place} of {}", other.name));
            }
            // Grounded, so that its doors and drawers have something fixed to open against.
            // To move the cabinet later, unground it first.
            run(
                "occurrence.ground",
                json!({"occurrence": made["occurrence"], "grounded": true}),
            )?;
            Ok(summary)
        }
        "joinery" => {
            // Screw or dowel holes at the corners of the selected cabinet, drilled inside its
            // component.
            let design = run(
                "document.inspect",
                json!({"bodies": false, "timeline": "summary"}),
            )?;
            let found = select::cabinet(&design, &run("inspect.measure", json!({}))?)?;
            let kind = params["type"].as_str().unwrap_or("confirmat");
            let commands = joinery::build(&found, &design, kind)?;
            let before = run("component.list", json!({"compact": true}))?["active"].clone();
            run("component.activate", json!({"component": found.name}))?;
            for (command, args) in commands {
                run(command, args)?;
            }
            run("component.activate", json!({"component": before}))?;
            Ok(json!({"cabinet": found.name, "type": kind, "joints": 4}))
        }
        "material" => {
            // A board material for the selected cabinet, or for just the selected parts.
            let design = run(
                "document.inspect",
                json!({"bodies": false, "timeline": false}),
            )?;
            let bodies = match params["scope"].as_str().unwrap_or("cabinet") {
                "parts" => select::picked(&design),
                "cabinet" => {
                    let measure = run("inspect.measure", json!({}))?;
                    select::parts(&measure, &select::cabinet(&design, &measure)?.name)
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
                json!({"bodies": false, "timeline": false}),
            )?;
            let imperial = matches!(design["units"].as_str(), Some("in" | "ft"));
            cutlist::cutlist(&run("inspect.measure", json!({}))?, imperial)
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
            crate::execute(&entry, &params, &mut run)
                .map(|v| v.to_string())
                .map_err(invalid)
        }
    }

    export!(Woodwork);
}

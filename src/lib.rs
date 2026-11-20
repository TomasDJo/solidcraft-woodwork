//! Woodworking plugin for SolidCraft. The modules are plain Rust that turn parameters into
//! app commands, so `cargo test` covers them; only `guest` touches the wasm bindings.

mod cabinet;
mod corner;
mod cutlist;
mod drawing;
mod dxf;
mod edit;
mod fittings;
mod joinery;
mod kit;
mod material;
mod nesting;
mod pdf;
mod select;
mod worktop;

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
    let built = match entry {
        "cabinet" => cabinet::build(params)?,
        _ => corner::build(params)?,
    };
    let (summary, made) = component(built, params, "Birch plywood", run)?;
    let name = summary["name"].as_str().unwrap_or_default();
    run(
        "component.metadata.set",
        json!({"component": name, "values": edit::remember(entry, params, name)}),
    )?;
    Ok((summary, made))
}

/// Runs a builder's commands in a new component named after it, inside whichever component
/// is active, gives its bodies the `material` field's board (`default` without one), and goes
/// back to the component the user was working in. Returns the summary and
/// `component.create`'s reply.
fn component(
    (commands, mut summary): (Commands, Value),
    params: &Value,
    default: &str,
    run: Run,
) -> Result<(Value, Value), String> {
    // Which copy of the active component the user is working in: a component can stand in
    // the design several times, and its id alone would bring back the first copy.
    let path = run("component.list", json!({"compact": true}))?["active_path"].clone();
    let made = run(
        "component.create",
        json!({"name": summary["name"], "grounded": false}),
    )?;
    summary["occurrence"] = made["occurrence"].clone();
    for (command, args) in commands {
        run(command, args)?;
    }
    let board = params["material"].as_str().unwrap_or(default);
    if board != "none" {
        let bodies = summary["bodies"].as_array().into_iter().flatten();
        let bodies: Vec<String> = bodies
            .filter_map(|b| b.as_str())
            .map(String::from)
            .collect();
        material::assign(run, board, &bodies)?;
    }
    // Back to where the user was working.
    let mut back = json!({"component": made["parent"]});
    if let Some(copy) = path.as_array().and_then(|path| path.last()) {
        back["occurrence"] = copy.clone();
        back["path"] = path.clone();
    }
    run("component.activate", back)?;
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
        json!({"bodies": false, "timeline": "summary"}),
    )?;
    // Beside it: the two cabinets' seats, their corners at the wall, are held together. That
    // follows a change of width or depth of either, and of a corner cabinet's return.
    let has = |seat: &str| {
        let features = design["timeline"].as_array().into_iter().flatten();
        features.into_iter().any(|feature| feature["name"] == seat)
    };
    let (theirs, ours) = match place {
        "right" => ("R", "L"),
        _ => ("L", "R"),
    };
    // At the end of a corner cabinet's return the placed cabinet stands a quarter turned.
    let against = [(false, 0), (true, if place == "right" { -90 } else { 90 })];
    let seats = (against.into_iter())
        .map(|(end, turn)| (kit::seat(&other.name, theirs, end), turn))
        .find(|(seat, _)| has(seat));
    let own = kit::seat(name, ours, false);
    if let (Some((seat, turn)), true, true) = (seats, has(&own), place != "above") {
        let joint = format!("{name} link");
        let mut args = json!({
            "type": "revolute", "name": joint, "quiet": true,
            "a": {"occurrence": format!("{}:1", other.name), "axis": seat},
            "b": {"occurrence": format!("{name}:1"), "axis": own},
        });
        if turn != 0 {
            args["angle"] = json!(format!("{turn} deg"));
        }
        run("joint.create", args)?;
        // A hinge that does not turn: the joint the app has between two axes.
        run("joint.limits", json!({"joint": joint, "min": 0, "max": 0}))?;
        return noted(other, place, name, run);
    }
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
    noted(other, place, name, run)
}

/// Notes a link on the placed cabinet's component, so Edit can make it again.
fn noted(other: &select::Cabinet, place: &str, name: &str, run: Run) -> Result<bool, String> {
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
    let about = run("component.metadata", json!({"component": found.name}))?;
    let mut ours = about["metadata"][edit::KEY].clone();
    let cabinet = ours["params"]["kind"].as_str().unwrap_or("standard");
    let (commands, origin) = joinery::build(found, &design, kind, cabinet)?;
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

/// Saves the cut list in `result` as a CSV or PDF file, a PDF of part labels, or a DXF file per
/// sheet, when its `file` field asks for one, and notes the name (the names for DXF) in the result.
pub fn save(entry: &str, params: &Value, result: &mut Value, write: Write) -> Result<(), String> {
    let refused = |name: &str, reason: String| {
        format!(
            "could not save {name}: {reason}. Saving needs a folder for cut lists, approved when the plugin is installed: install it again and enter a folder, or clear Save as"
        )
    };
    if let Some(name) = cutlist::file_name(entry, params)? {
        if params["format"] == "dxf" {
            let layouts = result["layouts"].as_array().cloned().unwrap_or_default();
            let names = dxf::files(&name[..name.len() - 4], &layouts);
            let trim = result["trim_mm"].as_f64().unwrap_or(0.0);
            for (layout, name) in layouts.iter().zip(&names) {
                let data = dxf::render(layout, trim);
                write(cutlist::FOLDER, name, data.as_bytes()).map_err(|r| refused(name, r))?;
            }
            result["saved"] = json!(names);
            return Ok(());
        }
        let data = match name.to_lowercase().strip_suffix(".pdf") {
            _ if params["format"] == "labels" => pdf::labels::render(result),
            Some(_) => pdf::render(result, &name[..name.len() - 4], params["diagrams"] != "no"),
            None => result["csv"]
                .as_str()
                .unwrap_or_default()
                .as_bytes()
                .to_vec(),
        };
        write(cutlist::FOLDER, &name, &data).map_err(|r| refused(&name, r))?;
        result["saved"] = json!(name);
    }
    Ok(())
}

pub fn execute(entry: &str, params: &Value, run: Run) -> Result<Value, String> {
    let summary = entries(entry, params, run)?;
    // A run that added or rebuilt a cabinet frames the model in the window (0.45; the
    // preview's dry runs leave the camera alone). Fit frames the selection when there is one,
    // such as the board a cabinet was placed against, so that is cleared first.
    if matches!(entry, "cabinet" | "corner" | "edit" | "worktop") {
        run("select.clear", json!({}))?;
        run("view.fit", json!({}))?;
    }
    Ok(summary)
}

fn entries(entry: &str, params: &Value, run: Run) -> Result<Value, String> {
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
                place = select::side(&picked, &measure);
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
        "worktop" => {
            // One board over the selected cabinets, in a component of its own, moved to the
            // point of the run that holds still and grounded: its parameters follow the cabinets'.
            let design = run(
                "document.inspect",
                json!({"bodies": false, "timeline": false}),
            )?;
            let measure = run("inspect.measure", json!({}))?;
            let mut params = params.clone();
            let name = params["name"].as_str().unwrap_or("Worktop");
            params["name"] = json!(kit::free_name(name, &design));
            let picked = select::picked(&params, &design);
            // Which of them is grounded or held to a cabinet outside the run, and how.
            let tree = run("component.list", json!({}))?;
            let mut held = serde_json::Map::new();
            for body in &picked {
                let Some(cabinet) = select::owner(&measure, body) else {
                    continue; // worktop::build says so
                };
                if !held.contains_key(cabinet) {
                    let grounded = edit::placement(&tree, cabinet).is_ok_and(|p| p.2);
                    let about = run("component.metadata", json!({"component": cabinet}))?;
                    let link = &about["metadata"][edit::KEY]["link"];
                    held.insert(cabinet.into(), json!({"grounded": grounded, "link": link}));
                }
            }
            params["_held"] = held.into();
            let built = worktop::build(&params, &picked, &measure, &design)?;
            let (summary, made) = component(built, &params, "Oak", run)?;
            // ponytail: no joint to the cabinets (Edit deletes a cabinet's joints): it stands
            // where the run's fixed point is now, and follows through its expressions only. A
            // cabinet ungrounded or relinked afterwards is not followed. A joint to a seat,
            // made again by Edit, would fix that.
            // plain numbers: this command takes mm whatever the design's unit is
            let mut to = json!({"occurrence": made["occurrence"], "space": "world", "translate": summary["at"]});
            if summary["turn"] != 0 {
                to["axis"] = json!([0, 0, 1]);
                to["angle"] = json!(format!("{} deg", summary["turn"]));
            }
            run("occurrence.move", to)?;
            run(
                "occurrence.ground",
                json!({"occurrence": made["occurrence"], "grounded": true}),
            )?;
            Ok(summary)
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
            // The worktops made by Worktop, whatever their names: by their length's comment.
            let all = design["params"].as_array().into_iter().flatten();
            let worktops: Vec<&str> = all
                .filter_map(|p| {
                    p["comment"]
                        .as_str()?
                        .strip_suffix(worktop::ALONG)?
                        .strip_suffix(": ")
                })
                .collect();
            let mut params = params.clone();
            params["_worktops"] = json!(worktops);
            let mut list = cutlist::cutlist(&measure, &local, imperial, &params)?;
            list["hardware"] = cutlist::hardware(&measure, &design);
            if params["format"] == "dxf" && cutlist::file_name(entry, &params)?.is_some() {
                // The holes of each drilled board, for the DXF files: one command a board.
                // ponytail: past the 256-command limit (less what this run used) the rest of
                // the boards get outlines only, named in `dxf_holes_skipped`.
                let mut left = 256usize.saturating_sub(4 + local.len());
                let drilled = |name: &str| {
                    let all = measure["bodies"].as_array().into_iter().flatten();
                    all.into_iter().any(|b| {
                        b["name"] == name && b["face_types"]["cylinder"].as_u64() > Some(0)
                    })
                };
                let (mut into_edge, mut skipped) = (0, Vec::new());
                let layouts = list["layouts"].as_array_mut().into_iter().flatten();
                for part in layouts.flat_map(|l| l["parts"].as_array_mut().into_iter().flatten()) {
                    let name = part["part"].as_str().unwrap_or_default().to_string();
                    if !drilled(&name) {
                        continue;
                    }
                    if left == 0 {
                        skipped.push(name);
                        continue;
                    }
                    left -= 1;
                    let faces = run("model.faces", json!({"body": name}))?;
                    let (holes, edge) = dxf::holes(&faces, part);
                    part["holes"] = json!(holes);
                    into_edge += edge;
                }
                list["dxf_edge_holes"] = json!(into_edge);
                if !skipped.is_empty() {
                    list["dxf_holes_skipped"] = json!(skipped);
                }
            }
            Ok(list)
        }
        "drawing" => {
            // A sheet per cabinet in the drawing Woodwork; a cabinet drawn again gets a new one.
            let design = run(
                "document.inspect",
                json!({"bodies": false, "timeline": false}),
            )?;
            let measure = run("inspect.measure", json!({}))?;
            let names = match params["scope"].as_str().unwrap_or("selected") {
                "selected" => {
                    vec![select::cabinet(&select::picked(params, &design), &measure)?.name]
                }
                "all" => drawing::cabinets(&measure),
                _ => return Err("scope must be selected or all".into()),
            };
            check(!names.is_empty(), "the design has no cabinets")?;
            // An open door's board is measured in its own component, as the cut list does.
            let turned: Vec<String> = (cutlist::turned(&measure).into_iter())
                .filter(|b| {
                    select::owner(&measure, b).is_some_and(|c| names.iter().any(|n| n == c))
                })
                .collect();
            drawing::budget(names.len(), turned.len())?;
            let tree = run("component.list", json!({"compact": true}))?;
            let joints = run("joint.list", json!({}))?;
            let mut local = std::collections::BTreeMap::new();
            for body in turned {
                let mesh = run("model.mesh", json!({"body": body, "tolerance": 1.0}))?;
                local.insert(body, cutlist::extents(&mesh));
            }
            // Worked out before the drawing is opened, which holds the model's commands back.
            let sheets: Vec<drawing::Steps> = (names.iter())
                .map(|name| drawing::sheet(&measure, &local, &tree, &joints, name))
                .collect::<Result<_, _>>()?;
            let list = run("drawing.list", json!({}))?;
            let found = (list["drawings"].as_array().into_iter().flatten())
                .find(|d| d["name"] == drawing::DRAWING);
            // In mm, as the board list, also in a design in inches (a drawing follows its
            // template's units, and the user may have changed them).
            let opened = match found {
                Some(found) => {
                    let opened = run("drawing.open", json!({"drawing": found["id"]}))?;
                    run("drawing.set", json!({"units": "mm"}))?;
                    opened
                }
                None => run(
                    "drawing.create",
                    json!({"name": drawing::DRAWING, "template": "iso-a3-assembly", "units": "mm"}),
                )?,
            };
            let stale = drawing::stale(&opened, &names, found.is_none());
            for steps in sheets {
                let mut ids = std::collections::BTreeMap::new();
                ids.insert("source".to_string(), opened["sources"][0]["id"].clone());
                for (command, args, keep) in steps {
                    let reply = run(command, drawing::fill(&args, &ids))?;
                    if !keep.is_empty() {
                        let id = reply.get("view").unwrap_or(&reply["sheet"]);
                        ids.insert(keep.to_string(), id.clone());
                    }
                }
            }
            for sheet in stale {
                run(
                    "drawing.sheet.delete",
                    json!({"sheet": sheet, "contents": "delete"}),
                )?;
            }
            run("drawing.close", json!({}))?;
            Ok(json!({"drawing": drawing::DRAWING, "sheets": names}))
        }
        _ => Err(format!("unknown entry `{entry}`")),
    }
}

#[cfg(test)]
mod tests;

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

//! Woodworking plugin for SolidCraft. The modules are plain Rust that turn parameters into
//! app commands, so `cargo test` covers them; only `guest` touches the wasm bindings.

mod cabinet;
mod corner;
mod cutlist;
mod drawing;
mod dxf;
mod edit;
mod fittings;
mod frame;
mod hardware;
mod joint;
mod kit;
mod material;
mod nesting;
mod panel;
mod pdf;
mod select;
mod settings;
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
/// `design` is `document.inspect`'s reply.
fn make(entry: &str, params: &Value, design: &Value, run: Run) -> Result<(Value, Value), String> {
    let built = build(entry, params)?;
    // The hinges' and slides' shared sizes, the first time a cabinet needs them.
    if hardware::used(&built.0) {
        for (command, args) in hardware::parameters(design) {
            run(command, args)?;
        }
    }
    // a frame is solid timber
    let default = if entry == "frame" {
        "Oak"
    } else {
        "Birch plywood"
    };
    let (summary, made) = component(built, params, default, run)?;
    let name = summary["name"].as_str().unwrap_or_default();
    run(
        "component.metadata.set",
        json!({"component": name, "values": edit::remember(entry, params, name)}),
    )?;
    Ok((summary, made))
}

/// The commands and summary of the entry that builds a cabinet or a frame.
fn build(entry: &str, params: &Value) -> Result<(Commands, Value), String> {
    match entry {
        "cabinet" => cabinet::build(params),
        "frame" => frame::build(params),
        _ => corner::build(params),
    }
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
    let tree = run("component.list", json!({"compact": true}))?;
    let path = tree["active_path"].clone();
    // The hardware models' definitions, at the root, the first time a cabinet copies them.
    if commands.iter().any(|c| c.0 == "occurrence.copy") {
        let definitions = hardware::definitions(&tree, &hardware::wanted(&commands));
        if !definitions.is_empty() {
            for (command, args) in definitions {
                run(command, args)?;
            }
            let mut back = json!({"component": tree["active"]});
            if let Some(copy) = path.as_array().and_then(|path| path.last()) {
                back["occurrence"] = copy.clone();
                back["path"] = path.clone();
            }
            run("component.activate", back)?;
        }
    }
    let made = run(
        "component.create",
        json!({"name": summary["name"], "grounded": false}),
    )?;
    summary["occurrence"] = made["occurrence"].clone();
    // A model's copy is moved into place (`hardware::copy`); one that cuts does so at the end.
    let mut cuts = Vec::new();
    for (command, mut args) in commands {
        if command != "occurrence.copy" {
            run(command, args)?;
            continue;
        }
        let matrix = args["_matrix"].take();
        let cut = args["_cut"].take();
        if let Some(args) = args.as_object_mut() {
            args.retain(|key, _| !key.starts_with('_'));
        }
        let (parent, model) = (args["parent"].clone(), args["component"].clone());
        let copy = run(command, args)?;
        run(
            "occurrence.move",
            json!({"occurrence": copy["occurrence"], "matrix": matrix}),
        )?;
        if !cut.is_null() {
            cuts.push((copy["occurrence"].clone(), parent, cut, model));
        }
    }
    if !cuts.is_empty() {
        let tree = run("component.list", json!({"compact": true}))?;
        let mut active = Value::Null;
        for (occurrence, door, name, model) in cuts {
            let body = model.as_str().unwrap_or_default();
            let tool = hardware::world_name(&tree, &occurrence, body)
                .ok_or("a hinge model's copy was not found")?;
            if active != door {
                run("component.activate", json!({"component": door}))?;
                active = door.clone();
            }
            run(
                "solid.combine",
                json!({
                    "target": door, "tools": [tool], "operation": "cut",
                    "keep_tools": true, "name": name,
                }),
            )?;
        }
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

/// Runs `commands` with the cabinet `name`'s component active. In a plugin run the app shifts
/// a hole's or a box's position by where the component stands, so a cabinet that has been
/// moved or turned is worked on at the origin, square, and put back.
fn inside(name: &str, commands: Commands, run: Run) -> Result<(), String> {
    let tree = run("component.list", json!({}))?;
    let before = tree["active"].clone();
    // ponytail: this leaves a cabinet that is not held to another grounded, as the cabinet
    // entry made it, also if the user had freed it since.
    let occurrence = format!("{name}:1");
    let (_, placed, _) = edit::placement(&tree, name)?;
    let square = json!([[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0]]);
    let shift = |to: &Value, run: Run| {
        let moved = run(
            "occurrence.move",
            json!({"occurrence": occurrence, "to_matrix": to}),
        )?;
        // a joint to something else puts it straight back, and the holes would miss
        check(
            moved["reverted"] != true,
            "the cabinet is held by a joint and cannot be moved to the origin to work on it",
        )
    };
    let away = (0..3).any(|r| {
        (0..4).any(|c| {
            (placed[r][c].as_f64().unwrap_or(0.0) - square[r][c].as_f64().unwrap_or(0.0)).abs()
                > 1e-6
        })
    });
    // Any link holds the cabinet where it is, its own to the cabinet it stands against and
    // those of the cabinets that stand against it: all are put aside for the move. Without
    // one of its own it is grounded instead.
    let links = edit::links(&tree, &run("joint.list", json!({}))?, name);
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
        shift(&square, run)?;
    }
    run("component.activate", json!({"component": name}))?;
    for (command, args) in commands {
        run(command, args)?;
    }
    run("component.activate", json!({"component": before}))?;
    if away {
        shift(&placed, run)?;
        free(false, run)?;
    }
    Ok(())
}

/// Makes a panel (`p`, its side settled) on a cabinet inside its component, gives it the
/// material of the cabinet's Side L, and adds it to the panels the cabinet remembers, so that
/// Edit makes it again. `measure` is `inspect.measure`'s reply from before (null in Edit, which
/// gives the materials back itself). Returns its bodies.
fn panel(cabinet: &str, p: &Value, measure: &Value, run: Run) -> Result<Vec<String>, String> {
    let design = run(
        "document.inspect",
        json!({"bodies": false, "timeline": false}),
    )?;
    let about = run("component.metadata", json!({"component": cabinet}))?;
    let mut ours = about["metadata"][edit::KEY].clone();
    let parts = select::parts(measure, cabinet);
    let (commands, record, bodies) = panel::build(p, cabinet, &ours, &parts, &design)?;
    inside(cabinet, commands, run)?;
    let side = format!("{cabinet} Side L");
    let all = measure["bodies"].as_array().into_iter().flatten();
    let board = all.into_iter().find(|b| b["name"] == side.as_str());
    if let Some(material) = board.and_then(|b| b["material"].as_str()) {
        if material != "Default" {
            run(
                "material.assign",
                json!({"bodies": bodies, "material": material}),
            )?;
        }
    }
    // One of the same side whose boards were deleted is replaced.
    let mut panels = ours["panels"].as_array().cloned().unwrap_or_default();
    panels.retain(|old| old["side"] != record["side"]);
    panels.push(record);
    ours["panels"] = json!(panels);
    run(
        "component.metadata.set",
        json!({"component": cabinet, "values": { edit::KEY: ours }}),
    )?;
    Ok(bodies)
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

/// Unix seconds, or 0 without the manifest's clock permission (and in unit tests).
fn now() -> u64 {
    #[cfg(target_arch = "wasm32")]
    return guest::craft::plugin::clock::now();
    #[cfg(not(target_arch = "wasm32"))]
    0
}

pub fn execute(entry: &str, params: &Value, run: Run) -> Result<Value, String> {
    // The hardware models are not boards nor part of a footprint: no function sees them.
    let mut run = |command: &str, args: Value| {
        let mut reply = run(command, args)?;
        if command == "inspect.measure" {
            hardware::hide(&mut reply);
        }
        Ok(reply)
    };
    let run: Run = &mut run;
    // Woodwork settings: the design-wide joinery and hardware of a new cabinet, and the camera.
    let ends = matches!(entry, "cabinet" | "corner" | "frame" | "edit" | "worktop");
    let stored = match ends || entry == "cutlist" {
        true => run("storage.get", json!({}))?,
        false => Value::Null,
    };
    let summary = entries(entry, params, &stored, run)?;
    // A run that added or rebuilt a cabinet frames the model in the window (0.45; the
    // preview's dry runs leave the camera alone). Fit frames the selection when there is one,
    // such as the board a cabinet was placed against, so that is cleared first. Woodwork
    // settings' Camera `leave` leaves the view as it is; the selection is still cleared, so the
    // next run does not place against a board picked before this one.
    if ends {
        run("select.clear", json!({}))?;
        if settings::fits(&stored) {
            run("view.fit", json!({}))?;
        }
    }
    Ok(summary)
}

fn entries(entry: &str, params: &Value, stored: &Value, run: Run) -> Result<Value, String> {
    match entry {
        "cabinet" | "corner" | "frame" => {
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
            if entry != "frame" {
                settings::design(entry, &mut params, stored);
            }
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
                // a frame stands on the floor, not on a cabinet's top
                if entry == "frame" && place == "above" {
                    place = "origin";
                }
                params["place"] = json!(place);
            }
            if entry == "corner" {
                // Settled here, so the cabinet remembers which it became: Edit builds it
                // again without knowing where it was placed.
                params["corner"] = json!(corner::hand(&params)?);
            }
            let (mut beside, mut beyond) = (None, false);
            // A name that is taken gets the next free number: Cabinet, Cabinet 2, Cabinet 3.
            let name = params["name"].as_str().unwrap_or("Cabinet");
            params["name"] = json!(kit::free_name(name, &design));
            if matching || place != "origin" {
                let mut found = select::cabinet(&picked, &measure)?;
                if matching {
                    params["_match"] = kit::matched(&found.name, &design)?;
                }
                // A panel on that side: the new cabinet stands beyond it, not linked, since the
                // link holds the two sides' seats together.
                beyond = matches!(place, "left" | "right") && found.with_panel(&measure, place);
                if place != "origin" {
                    beside = Some(found);
                }
            }
            let (mut summary, made) = make(entry, &params, &design, run)?;
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
                held = !beyond && link(&other, place, &name, at, run)?;
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
            // a cabinet or a frame
            let name = select::piece(&select::picked(params, &design), &measure)?;
            let about = run("component.metadata", json!({"component": name}))?;
            let ours = &about["metadata"][edit::KEY];
            let changes = settings::edit_plinth(params, ours, &design, stored);
            let (entry, params) =
                edit::merge(ours, &changes, &design).map_err(|e| format!("`{name}`: {e}"))?;
            // Checked before anything is taken away.
            build(&entry, &params)?;
            let tree = run("component.list", json!({}))?;
            let (parent, matrix, grounded) = edit::placement(&tree, &name)?;
            let materials = edit::materials(&measure, &select::parts(&measure, &name));
            // The cabinets held to this one lose their link with it: found now, held again after.
            let joints = run("joint.list", json!({}))?;
            let hanging = edit::hanging(&tree, &joints, &name);
            run("component.activate", json!({"component": parent}))?;
            let gone = run("component.delete", json!({"component": name}))?;
            let (mut summary, made) = make(&entry, &params, &design, run)?;
            // Its panels, those whose boards were still there; they get their material back
            // with the rest below.
            let parts = select::parts(&measure, &name);
            let panels = about["metadata"][edit::KEY]["panels"].as_array().cloned();
            for p in panels.into_iter().flatten() {
                if parts.contains(&panel::body(&name, &p)) {
                    let bodies = panel(&name, &p, &Value::Null, run)?;
                    if let Some(all) = summary["bodies"].as_array_mut() {
                        all.extend(bodies.into_iter().map(Value::from));
                    }
                }
            }
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
            // (read per cabinet: a frame has none, and no Side L to read)
            for other in hanging {
                let this = select::named(&measure, &name)?;
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
                    select::parts(&measure, &select::piece(&picked, &measure)?)
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
        "panel" => {
            // An end panel or a filler on one end of the selected cabinet, inside
            // its component.
            let design = run(
                "document.inspect",
                json!({"bodies": false, "timeline": false}),
            )?;
            let measure = run("inspect.measure", json!({}))?;
            let picked = select::picked(params, &design);
            let name = select::cabinet(&picked, &measure)?.name;
            let board = picked[0].get(name.len() + 1..).unwrap_or_default();
            let side = panel::side(params, board)?;
            // Not where another cabinet stands: the one this was placed against, or one
            // placed against this. Their link notes say which side; this one's own only while
            // its link is still there (the note stays when the joint goes).
            let about = run("component.metadata", json!({"component": name}))?;
            let tree = run("component.list", json!({}))?;
            let joints = run("joint.list", json!({}))?;
            let held = edit::links(&tree, &joints, &name).iter().any(|l| l.1);
            let place = about["metadata"][edit::KEY]["link"]["place"].as_str();
            let mut taken = vec![match place.filter(|_| held) {
                Some("right") => "left",
                Some("left") => "right",
                _ => "",
            }];
            for other in edit::hanging(&tree, &joints, &name) {
                let about = run("component.metadata", json!({"component": other}))?;
                let place = about["metadata"][edit::KEY]["link"]["place"].as_str();
                taken.push(match place {
                    Some("right") => "right",
                    Some("left") => "left",
                    _ => "",
                });
            }
            check(
                !taken.contains(&side),
                &format!(
                    "`{name}` has a cabinet against its {side}: a panel there would stand in it"
                ),
            )?;
            let mut params = params.clone();
            params["side"] = json!(side);
            let bodies = panel(&name, &params, &measure, run)?;
            Ok(json!({"cabinet": name, "side": side, "bodies": bodies}))
        }
        "worktop" => {
            // A worktop over the selected cabinets, in a component of its own on the origin.
            // Run again it replaces the worktop of that name, and any worktop picked.
            let design = run(
                "document.inspect",
                json!({"bodies": false, "timeline": false}),
            )?;
            let measure = run("inspect.measure", json!({}))?;
            let tree = run("component.list", json!({}))?;
            let components: Vec<&str> = (tree["components"].as_array().into_iter().flatten())
                .filter_map(|c| c["name"].as_str())
                .collect();
            // A component that is a worktop: the cabinets and the values it remembers.
            type Remembered = Option<(Vec<String>, Value)>;
            let mut remembered = |component: &str| -> Result<Remembered, String> {
                if !components.contains(&component) {
                    return Ok(None);
                }
                let about = run("component.metadata", json!({"component": component}))?;
                let ours = &about["metadata"][edit::KEY];
                Ok((ours["entry"] == "worktop").then(|| {
                    let all = ours["cabinets"].as_array().into_iter().flatten();
                    let cabinets = all.filter_map(|c| c.as_str().map(String::from)).collect();
                    (cabinets, ours["params"].clone())
                }))
            };
            let (mut cabinets, mut old) = (Vec::<String>::new(), Vec::<String>::new());
            let (mut kept, mut any_cabinet) = (None, false);
            for body in select::picked(params, &design) {
                if let Some(cabinet) = select::owner(&measure, &body) {
                    cabinets.push(cabinet.to_string());
                    any_cabinet = true;
                    continue;
                }
                let mut found = false;
                for c in worktop::owners(&body, &components) {
                    if let Some((theirs, values)) = remembered(c)? {
                        old.push(c.to_string());
                        cabinets.extend(theirs);
                        kept.get_or_insert(values);
                        found = true;
                        break;
                    }
                }
                check(
                    found,
                    &format!("`{body}` is not part of a cabinet or a worktop"),
                )?;
            }
            // Only a worktop picked: made again as it was, its name and options kept.
            let mut params = match (kept, any_cabinet) {
                (Some(values), false) => values,
                _ => params.clone(),
            };
            // its material and thickness from Woodwork settings, unless it remembers them
            settings::design(entry, &mut params, stored);
            let name = params["name"]
                .as_str()
                .unwrap_or("Worktop")
                .trim()
                .to_string();
            let name = match remembered(&name)? {
                Some((theirs, values)) => {
                    // Run again with nothing picked: as it was made, over the same cabinets.
                    if cabinets.is_empty() {
                        cabinets = theirs;
                        params = values;
                    }
                    old.push(name.clone());
                    name
                }
                None => {
                    let free = kit::free_name(&name, &design);
                    check(
                        !components.contains(&free.as_str()),
                        &format!("name: `{free}` is taken; choose another"),
                    )?;
                    free
                }
            };
            params["name"] = json!(name);
            // A cabinet (or frame) deleted since a worktop remembered it is left out.
            cabinets
                .retain(|c| select::named(&measure, c).is_ok() || worktop::is_frame(&measure, c));
            let mut seen = std::collections::BTreeSet::new();
            cabinets.retain(|c| seen.insert(c.clone()));
            // Checked before anything is taken away.
            let built = worktop::build(&params, &cabinets, &measure)?;
            let bodies = (measure["bodies"].as_array().into_iter().flatten())
                .filter_map(|b| b["name"].as_str());
            for taken in bodies.filter(|b| !old.iter().any(|w| worktop::piece_of(b, w))) {
                check(
                    !built.1["bodies"]
                        .as_array()
                        .is_some_and(|all| all.contains(&json!(taken))),
                    &format!("name: a body `{taken}` is already in the design; choose another"),
                )?;
            }
            old.sort();
            old.dedup();
            for component in &old {
                run("component.delete", json!({"component": component}))?;
            }
            // ponytail: made in the active component at world coordinates, so that component
            // must stand on the origin, unturned, as a room component normally does.
            let (mut summary, made) = component(built, &params, "Oak", run)?;
            // The tool's shelf and battens are plywood boards, whatever the worktop is.
            if let Some(parts) = summary["tool"]["bodies"].as_array() {
                let parts: Vec<String> = parts
                    .iter()
                    .filter_map(|b| b.as_str())
                    .map(String::from)
                    .collect();
                material::assign(run, "Birch plywood", &parts)?;
            }
            // The cabinet's top (or a sink base's rails) gets the opening too, in its own
            // component, so the tool drops through.
            // ponytail: a feature of the cabinet that it does not remember: Edit cabinet makes
            // the top whole again (run Worktop again), and a tool moved on a run again leaves
            // the old opening in the top.
            let cut = &summary["tool"]["cut"];
            if let (Some(cabinet), Some(targets)) = (
                summary["tool"]["cabinet"].as_str(),
                cut["targets"].as_array().filter(|t| !t.is_empty()),
            ) {
                let world = ["min", "max"]
                    .map(|end| [0, 1, 2].map(|i| cut[end][i].as_f64().unwrap_or_default()));
                let tree = run("component.list", json!({}))?;
                let (corner, size) = worktop::local(world, &edit::placement(&tree, cabinet)?.1);
                let args = json!({
                    "length": kit::mm(size[0]), "width": kit::mm(size[1]), "height": kit::mm(size[2]),
                    "corner": corner, "operation": "cut", "targets": targets,
                    "name": format!("{name} tool opening in {cabinet}"),
                });
                inside(cabinet, vec![("solid.box", args)], run)?;
            }
            run(
                "occurrence.ground",
                json!({"occurrence": made["occurrence"], "grounded": true}),
            )?;
            let mut values = edit::remember("worktop", &params, &name);
            values[edit::KEY]["cabinets"] = json!(cabinets);
            run(
                "component.metadata.set",
                json!({"component": name, "values": values}),
            )?;
            if !old.is_empty() {
                summary["replaced"] = json!(old);
            }
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
            let mut params = params.clone();
            settings::design(entry, &mut params, stored);
            params["_notched"] = json!(cutlist::notched(&design));
            let params = &params;
            let mut list = cutlist::cutlist(&measure, &local, imperial, params)?;
            list["hardware"] = cutlist::hardware(&measure, &design);
            if params["format"] == "dxf" && cutlist::file_name(entry, params)?.is_some() {
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
            // All: only the cabinets whose sheet would change, and as many as fit in this run.
            let start = now();
            let used = std::cell::Cell::new(0);
            let run = &mut |command: &str, args: Value| {
                used.set(used.get() + 1);
                run(command, args)
            };
            let design = run(
                "document.inspect",
                json!({"bodies": false, "timeline": "summary"}),
            )?;
            let mut measure = run("inspect.measure", json!({}))?;
            // the board list notes them as the cut list does
            measure["notched"] = json!(cutlist::notched(&design));
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
            drawing::budget(turned.len())?;
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
            let all = params["scope"] == "all";
            let mut todo = (names.iter().zip(sheets))
                .map(|(name, steps)| (name, drawing::fingerprint(&steps), steps))
                .filter(|(name, print, _)| !all || !drawing::up_to_date(&opened, name, print))
                .peekable();
            let (mut drawn, mut gone, mut noted) = (Vec::new(), Vec::new(), json!({}));
            // A title block that does not weigh the cabinet, or each open of the drawing does.
            // ponytail: the first sheet's block, the template's unless the user changed it.
            let block = (opened["sheets"][0]["title_block"].is_object())
                .then(|| drawing::unweighed(&opened["sheets"][0]["title_block"]));
            // At least one sheet a run, so that running it again always gets on.
            while let Some((name, print, steps)) = todo.next_if(|(_, _, steps)| {
                let elapsed = now().saturating_sub(start);
                drawn.is_empty() || drawing::fits(elapsed, used.get(), steps.len() + 1)
            }) {
                let mut ids = std::collections::BTreeMap::new();
                ids.insert("source".to_string(), opened["sources"][0]["id"].clone());
                for (command, mut args, keep) in steps {
                    if let (true, Some(block)) = (command == "drawing.sheet.add", block.as_ref()) {
                        args["title_block"] = block.clone();
                    }
                    let reply = run(command, drawing::fill(&args, &ids))?;
                    if !keep.is_empty() {
                        let id = reply.get("view").unwrap_or(&reply["sheet"]);
                        ids.insert(keep.to_string(), id.clone());
                    }
                }
                // Its old sheet goes now, so a run that stops leaves one sheet a cabinet.
                for sheet in drawing::stale(&opened, std::slice::from_ref(name), false) {
                    run(
                        "drawing.sheet.delete",
                        json!({"sheet": sheet, "contents": "delete"}),
                    )?;
                    gone.push(sheet);
                }
                if let Some(id) = ids["sheet"].as_str() {
                    noted[id] = json!(print);
                }
                drawn.push(name.clone());
            }
            let remaining: Vec<String> = todo.map(|(name, _, _)| name.clone()).collect();
            for sheet in drawing::stale(&opened, &[], found.is_none()) {
                run(
                    "drawing.sheet.delete",
                    json!({"sheet": sheet, "contents": "delete"}),
                )?;
            }
            // The fingerprints of the sheets still there, and of those just drawn.
            let old = opened["properties"][drawing::KEY].as_object().cloned();
            for (id, print) in old.into_iter().flatten() {
                let kept = (opened["sheets"].as_array().into_iter().flatten())
                    .any(|s| s["id"] == id.as_str() && !gone.contains(&s["id"]));
                if kept && noted.get(&id).is_none() {
                    noted[id] = print;
                }
            }
            run(
                "drawing.properties.set",
                json!({"key": drawing::KEY, "value": noted}),
            )?;
            run("drawing.close", json!({}))?;
            let s = |n: usize| if n == 1 { "" } else { "s" };
            let message = match (drawn.len(), remaining.len()) {
                (0, 0) => "All sheets were up to date".to_string(),
                (d, 0) if d == names.len() => format!("Drew {d} sheet{}", s(d)),
                (d, 0) => format!("Drew {d} sheet{}; the rest were up to date", s(d)),
                (d, r) => format!(
                    "Drew {d} of {} sheets; run Drawing again for the rest",
                    d + r
                ),
            };
            Ok(
                json!({"drawing": drawing::DRAWING, "sheets": drawn, "remaining": remaining,
                      "message": message}),
            )
        }
        "settings" => {
            let stored = run("storage.get", json!({}))?;
            run(
                "storage.patch",
                json!({"patch": settings::patch(params, &stored)?}),
            )?;
            let stored = run("storage.get", json!({}))?;
            // See-through boards: our materials made so, or solid again, now.
            let changed = material::see_through(run, material::opacity(&stored))?;
            Ok(json!({"settings": stored["document"]["settings"], "see_through": changed}))
        }
        _ => Err(format!("unknown entry `{entry}`")),
    }
}

/// The dialog hook (`dialog@0.1`): the Cabinet and Corner dialogs' defaults from Woodwork
/// settings. It runs outside a plugin run, so `storage.get` names our plugin. Edit's: the
/// fields the selected cabinet does not take, hidden.
pub fn dialog(entry: &str, values: &Value, run: Run) -> Result<Value, String> {
    if entry == "edit" {
        // A failing hook fails `plugin.run`: if the design cannot be read, hide nothing.
        let design = run(
            "document.inspect",
            json!({"bodies": "names", "timeline": false}),
        );
        return Ok(design.map_or(json!({}), |d| edit::hidden(values, &d)));
    }
    let stored = run("storage.get", json!({"plugin": "woodwork"}))?;
    Ok(settings::fields(entry, values, &stored))
}

#[cfg(test)]
mod tests;

#[cfg(target_arch = "wasm32")]
mod guest {
    wit_bindgen::generate!({ path: "wit", world: "command-dialog-plugin" });

    use craft::plugin::types::{FieldFailure, PluginError};

    struct Woodwork;

    fn invalid(reason: String) -> PluginError {
        PluginError::InvalidInput(FieldFailure {
            field: "params".into(),
            reason,
        })
    }

    fn run(command: &str, args: serde_json::Value) -> Result<serde_json::Value, String> {
        let reply =
            craft::plugin::commands::run(command, &args.to_string()).map_err(|e| match e {
                PluginError::CommandFailed(f) => format!("{}: {}", f.command, f.reason),
                other => format!("{command}: {other:?}"),
            })?;
        serde_json::from_str(&reply).map_err(|e| format!("{command}: bad reply: {e}"))
    }

    impl exports::craft::plugin::dialog_provider::Guest for Woodwork {
        fn update_fields(entry: String, values: String) -> Result<String, PluginError> {
            let values = serde_json::from_str(&values).map_err(|e| invalid(e.to_string()))?;
            let reply = crate::dialog(&entry, &values, &mut run).map_err(invalid)?;
            Ok(reply.to_string())
        }
    }

    impl exports::craft::plugin::entry::Guest for Woodwork {
        fn execute(entry: String, params: String) -> Result<String, PluginError> {
            let params = serde_json::from_str(&params).map_err(|e| invalid(e.to_string()))?;
            let mut result = crate::execute(&entry, &params, &mut run).map_err(invalid)?;
            let mut write = |folder: &str, name: &str, data: &[u8]| {
                craft::plugin::files::write_file(folder, name, data).map_err(|e| format!("{e:?}"))
            };
            crate::save(&entry, &params, &mut result, &mut write).map_err(invalid)?;
            // The app's status line only says Done: the message goes to its log too.
            if let Some(message) = result["message"].as_str() {
                craft::plugin::log::log(craft::plugin::log::Level::Info, message);
            }
            Ok(result.to_string())
        }
    }

    export!(Woodwork);
}

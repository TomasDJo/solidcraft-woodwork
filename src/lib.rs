//! Woodworking plugin for SolidCraft. The modules are plain Rust that turn parameters into
//! app commands, so `cargo test` covers them; only `guest` touches the wasm bindings.

mod base;
mod cabinet;
mod corner;
mod cost;
mod cutlist;
mod drawing;
mod dxf;
mod edit;
mod fittings;
mod hardware;
mod joint;
mod kit;
mod line;
mod material;
mod nesting;
mod panel;
mod pdf;
mod select;
mod settings;
mod worktop;

use kit::{check, num};
use serde_json::{json, Map, Value};

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
    let (summary, made) = component(built, params, "Birch plywood", run)?;
    let name = summary["name"].as_str().unwrap_or_default();
    let remembered = edit::remember(entry, params, name);
    // What Edit's dialog shows for it, in a file of ours in the design (`edit::blob`), which its
    // hook can read (a read entry may not read component metadata: it counts as an edit
    // command). A file, not our main document: every hook parses that, with 10M fuel.
    let parts = summary["bodies"].as_array().into_iter().flatten();
    let copy = edit::copy(entry, params, name, parts.filter_map(Value::as_str));
    run(
        "component.metadata.set",
        json!({"component": name, "values": remembered}),
    )?;
    run("_put_blob", json!({"name": edit::blob(name), "data": copy}))?;
    Ok((summary, made))
}

/// The commands and summary of the builder of a box (`cabinet`) or corner cabinet.
fn build(entry: &str, params: &Value) -> Result<(Commands, Value), String> {
    match entry {
        "cabinet" => cabinet::build(params),
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
        let (glass, bodies): (Vec<String>, Vec<String>) = bodies
            .filter_map(|b| b.as_str())
            .map(String::from)
            .partition(|b| fittings::is_glass(b));
        material::assign(run, board, &bodies)?;
        if !glass.is_empty() {
            material::assign(run, "Glass", &glass)?;
        }
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

/// Apply settings (the `rebuild` job, 0.48: no 30 s): each picked cabinet (a box or corner
/// cabinet), or every one with none picked (`all`), built again (`rebuild`) with the settings
/// where its values differ (`settings::apply`), or as it was made (`settings` false), each
/// reported as `_progress` (`control.progress` in the guest).
fn apply(
    params: &Value,
    stored: &Value,
    all: bool,
    settings: bool,
    run: Run,
) -> Result<Value, String> {
    let design = run(
        "document.inspect",
        json!({"bodies": false, "timeline": false}),
    )?;
    let names_only = body_names(run)?;
    // The cabinets of the picked bodies: a board, or a hinge or slide model's copy (`HW hinge
    // (A:1/A Door:1/HW hinge:2)`, which measures do not show: the first name on its path).
    let bodies = match all {
        // every cabinet: its Side L
        true => (names_only["bodies"].as_array().into_iter().flatten())
            .filter_map(|b| b["name"].as_str().filter(|n| n.ends_with(" Side L")))
            .map(String::from)
            .collect(),
        false => select::picked(params, &design),
    };
    // as the dialog hook says it (`select::applies_to`)
    let cabinets: Vec<String> = (names_only["bodies"].as_array().into_iter().flatten())
        .filter_map(|b| b["name"].as_str()?.strip_suffix(" Side L"))
        .map(kit::prefix)
        .collect();
    let (names, skipped) = select::cabinets_of(&cabinets, &bodies, false);
    let mut skipped: Vec<Value> = (skipped.into_iter())
        .map(|body| json!({"picked": body, "why": "not part of a cabinet"}))
        .collect();
    // Measured only what is built again: its boards' materials, and its panels.
    let measure = measured(names_only, &names, run)?;
    let mut todo = Vec::new();
    for name in names {
        let about = run("component.metadata", json!({"component": name}))?;
        let ours = &about["metadata"][edit::KEY];
        let Some(entry) = ours["entry"]
            .as_str()
            .filter(|e| matches!(*e, "cabinet" | "corner"))
        else {
            skipped.push(json!({"cabinet": name, "why": "not a box or corner cabinet"}));
            continue;
        };
        let entry = entry.to_string();
        let (_, base) = edit::merge(ours, &json!({}), &design)?;
        match settings {
            false => todo.push((name, about, entry, base, vec![])),
            true => match settings::apply(&entry, &base, stored) {
                Some((params, changed)) => todo.push((name, about, entry, params, changed)),
                // along a line: built again all the same, to read the line again
                None if along(&base) => todo.push((name, about, entry, base, vec!["line".into()])),
                None => skipped.push(json!({"cabinet": name, "why": "already has these settings"})),
            },
        }
    }
    let (mut applied, mut changes, mut lines) = (Vec::new(), Map::new(), Map::new());
    let count = todo.len();
    for (i, (name, about, entry, params, changed)) in todo.into_iter().enumerate() {
        let message = format!("{name} ({} of {count})", i + 1);
        run(
            "_progress",
            json!({"fraction": i as f64 / count as f64, "message": message}),
        )?;
        let design = run(
            "document.inspect",
            json!({"bodies": false, "timeline": false}),
        )?;
        let built = rebuild(&name, &about, &entry, &params, &design, &measure, run)?;
        // along a line: what it read, or why it was left where it stood
        if let Some(said) = built["line"].as_str() {
            lines.insert(name.clone(), json!(said));
        }
        changes.insert(name.clone(), json!(changed));
        applied.push(name);
    }
    let with = if settings {
        "with the settings"
    } else {
        "as they were made"
    };
    let mut message = format!("{} cabinets built again {with}", applied.len());
    if !skipped.is_empty() {
        message += &format!("; {} skipped (see skipped)", skipped.len());
    }
    Ok(
        json!({"applied": applied, "changed": changes, "lines": lines, "skipped": skipped,
        "message": message}),
    )
}

/// Builds the cabinet `name` again from `params` (`edit::merge`'s), where it stands: the same
/// name, place, links, the materials of the boards that had another than the rest, and its
/// panels. `about` is its `component.metadata`, `design` and `measure` read before. Edit, and
/// Apply settings.
fn rebuild(
    name: &str,
    about: &Value,
    entry: &str,
    params: &Value,
    design: &Value,
    measure: &Value,
    run: Run,
) -> Result<Value, String> {
    let name = name.to_string();
    let tree = run("component.list", json!({}))?;
    let (parent, matrix, grounded) = edit::placement(&tree, &name)?;
    let materials = edit::materials(measure, &select::parts(measure, &name));
    // The cabinets held to this one lose their link with it: found now, held again after.
    let joints = run("joint.list", json!({}))?;
    let hanging = edit::hanging(&tree, &joints, &name);
    // Sized by its drawers' inside: its outside again from that, whatever changed the boxes.
    let mut params = params.clone();
    if entry == "cabinet" && params["sizing"] == "drawer" {
        match params["drawers"].as_f64().unwrap_or(0.0) > 0.0 {
            true => cabinet::from_drawers(&mut params)?,
            false => params["sizing"] = json!("outside"),
        }
    }
    // Along a line: read it again (Apply settings follows an edge or an undimensioned line so).
    // A line that can no longer be read leaves the cabinet where it stands, and says why.
    let found = match along(&params) {
        true => Some(along_line(&mut params, design, run)),
        false => None,
    };
    let params = &params;
    run("component.activate", json!({"component": parent}))?;
    let gone = run("component.delete", json!({"component": name}))?;
    let (mut summary, made) = make(entry, params, design, run)?;
    // Its panels, those whose boards were still there; they get their material back
    // with the rest below.
    let parts = select::parts(measure, &name);
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
    let mut placed = false;
    match found {
        Some(Ok(found)) => {
            place_along(&made["occurrence"], &mut summary, params, &found, run)?;
            placed = true;
        }
        Some(Err(why)) => summary["line"] = json!(format!("{why}: left where it stood")),
        None => {}
    }
    if !placed {
        run(
            "occurrence.move",
            json!({"occurrence": made["occurrence"], "to_matrix": matrix, "move_grounded": true}),
        )?;
    }
    // Held again to the cabinet it stood against, when that is still there; and the
    // cabinets that were held to it, to the new one.
    let stands = |matrix: &Value| [0, 1, 2].map(|i| matrix[i][3].as_f64().unwrap_or(0.0));
    let was = &about["metadata"][edit::KEY]["link"];
    // This cabinet and the one it was held to: what the links read.
    let mut linked = vec![name.clone()];
    linked.extend(was["to"].as_str().map(String::from));
    let measure = measured(body_names(run)?, &linked, run)?;
    let mut held = false;
    if let (Some(to), Some(place)) = (was["to"].as_str(), was["place"].as_str()) {
        if let Ok(other) = select::named(&measure, to) {
            held = link(&other, place, &name, stands(&matrix), run)?;
        }
    }
    if !held && !placed {
        run(
            "occurrence.ground",
            json!({"occurrence": made["occurrence"], "grounded": grounded}),
        )?;
    }
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
    Ok(summary)
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

/// Every body's name, from `inspect.measure`'s `names_only`: nothing is measured. A whole
/// kitchen's full measure takes over a second, this a few ms. What `select` finds by name
/// (`owner`, `parts`, `piece`) reads it as it is.
fn body_names(run: Run) -> Result<Value, String> {
    run("inspect.measure", json!({"names_only": true}))
}

/// `names` (`body_names`) with the bodies of `cabinets` measured (boxes, materials, volumes),
/// the others by name only: what placing, Edit, a panel, a worktop and Apply settings read.
fn measured(names: Value, cabinets: &[String], run: Run) -> Result<Value, String> {
    // each body once: the app refuses a name given twice
    let mut wanted: Vec<String> = Vec::new();
    for body in cabinets.iter().flat_map(|c| select::parts(&names, c)) {
        if !wanted.contains(&body) {
            wanted.push(body);
        }
    }
    if wanted.is_empty() {
        return Ok(names);
    }
    let mut measure = run("inspect.measure", json!({"bodies": wanted}))?;
    let done: Vec<Value> = (measure["bodies"].as_array().into_iter().flatten())
        .map(|b| b["name"].clone())
        .collect();
    let rest = (names["bodies"].as_array().into_iter().flatten())
        .filter(|b| !done.contains(&b["name"]))
        .cloned();
    let rest: Vec<Value> = rest.collect();
    match measure["bodies"].as_array_mut() {
        Some(bodies) => bodies.extend(rest),
        None => measure["bodies"] = json!(rest),
    }
    Ok(measure)
}

/// `measured` for the cabinets of the `picked` bodies.
fn picked_measure(picked: &[String], run: Run) -> Result<Value, String> {
    let names = body_names(run)?;
    let mut owners: Vec<String> = Vec::new();
    for body in picked {
        if let Some(owner) = select::owner(&names, body).map(String::from) {
            if !owners.contains(&owner) {
                owners.push(owner);
            }
        }
    }
    measured(names, &owners, run)
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
    // Cabinet's Mode Edit picked is Edit (one dialog for a new cabinet and a change to one);
    // Auto goes by what is picked (`select::auto`, as the dialog hook does).
    let mut params = params.clone();
    let mut entry = match entry == "cabinet" && params["mode"] == "edit" {
        true => "edit",
        false => entry,
    };
    if entry == "cabinet" && params["mode"] == "auto" {
        let design = run(
            "document.inspect",
            json!({"bodies": false, "timeline": false}),
        )?;
        let measure = run("inspect.measure", json!({}))?;
        let picked = select::picked(&params, &design);
        match select::auto(&measure, &picked) {
            select::Auto::Edit(..) => entry = "edit",
            _ => {
                // the Edit fields the dialog hid but sent: a new cabinet takes the settings'
                if let Some(all) = params.as_object_mut() {
                    for key in edit::EDIT_ONLY {
                        all.remove(key);
                    }
                }
            }
        }
    }
    let params = &params;
    // Woodwork settings: the design-wide joinery and hardware of a new cabinet, and the camera.
    let ends = matches!(entry, "cabinet" | "corner" | "edit" | "worktop");
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

/// mm per unit of the dialog's `units`.
fn unit_mm(params: &Value) -> f64 {
    if params["units"] == "in" {
        25.4
    } else {
        1.0
    }
}

/// Place: Along a line (#147). Reads the picked sketch line, edge or wall face (`line`), and
/// with Width from line sets the width, with a Height from line pick (`height_line`) the height:
/// over a sketch line's driving length dimension when it has one (named first if the app named
/// it `d<n>`), so the cabinet follows it with no rebuild; else read now, and again by Apply
/// settings. Returns the line, the way the front faces, and what the summary says about it.
fn along_line(
    params: &mut Value,
    design: &Value,
    run: Run,
) -> Result<(line::Line, [f64; 2], String), String> {
    let picked = params["line"][0].clone();
    let tall = params["height_line"][0].clone();
    // A command that fails ends the whole run, so a pick that is gone (Apply settings reads
    // a remembered one) is refused here, where the caller can leave the cabinet standing.
    let bodies = match [&picked, &tall].iter().any(|p| p["body"].is_string()) {
        true => body_names(run)?,
        false => Value::Null,
    };
    for pick in [&picked, &tall].into_iter().filter(|p| !p.is_null()) {
        there(pick, design, &bodies)?;
    }
    let index = picked["index"].as_u64().unwrap_or(u64::MAX);
    let (line, sketch) = match picked["type"].as_str() {
        Some("external_sketch_curve") => {
            let sketch = run("sketch.inspect", json!({"sketch": picked["sketch"]}))?;
            let curve = picked["id"].as_str().unwrap_or_default();
            let found = line::from_sketch(&sketch, curve)?;
            (found, Some((sketch, curve.to_string())))
        }
        Some(kind @ ("edge" | "face")) => {
            let body = json!({"body": picked["body"]});
            let edges = run("model.edges", body.clone())?;
            let faces = run("model.faces", body)?;
            let found = match kind {
                "edge" => line::from_edge(&edges, &faces, index)?,
                _ => line::from_face(&edges, &faces, index)?,
            };
            (found, None)
        }
        _ => {
            return Err(
                "Along a line: pick a line (a sketch line, a model edge or a wall face)".into(),
            )
        }
    };
    let front = line::facing(&line, params["faces"] == "flip");
    let start = params["start_at"].as_str().unwrap_or("left").to_string();
    check(
        matches!(start.as_str(), "left" | "right" | "middle"),
        "start_at must be left, right or middle",
    )?;
    let scale = unit_mm(params);
    let unit = if scale == 1.0 { "mm" } else { "in" };
    let offset = num(params, "offset", 0.0)?;
    let mut said = Vec::new();
    if params["width_from_line"] == true {
        // Drawer sizing works the width out from the boxes' inside: one or the other.
        check(
            params["sizing"] != "drawer",
            "Width from line and Size by the drawers' inside don't go together: pick one",
        )?;
        let width = line::width(&line, &start, offset * scale);
        check(width > 0.0, "the line is shorter than the offset")?;
        params["width"] = json!((width / scale * 1e6).round() / 1e6);
        params["sizing"] = json!("outside");
        match follow(line.dimension.clone(), sketch, design, run)? {
            Some(name) => {
                let off = if start == "middle" {
                    2.0 * offset.abs()
                } else {
                    offset
                };
                params["_line"]["width"] = json!(match off == 0.0 {
                    true => name.clone(),
                    false => format!("{name} - {off} {unit}"),
                });
                said.push(format!("width follows {name}"));
            }
            None => said.push("width read from the line".to_string()),
        }
    }
    if !tall.is_null() {
        let (length, dimension, sketch) = match tall["type"].as_str() {
            Some("external_sketch_curve") => {
                let sketch = run("sketch.inspect", json!({"sketch": tall["sketch"]}))?;
                let curve = tall["id"].as_str().unwrap_or_default().to_string();
                let (length, dimension) = line::sketch_length(&sketch, &curve)?;
                (length, dimension, Some((sketch, curve)))
            }
            Some("edge") => {
                let edges = run("model.edges", json!({"body": tall["body"]}))?;
                let index = tall["index"].as_u64().unwrap_or(u64::MAX);
                (line::edge_length(&edges, index)?, None, None)
            }
            _ => return Err("Height from line: pick a sketch line or an edge".into()),
        };
        params["height"] = json!((length / scale * 1e6).round() / 1e6);
        match follow(dimension, sketch, design, run)? {
            Some(name) => {
                params["_line"]["height"] = json!(name);
                said.push(format!("height follows {name}"));
            }
            None => said.push("height read from the line".to_string()),
        }
    }
    let mut message = String::from("Placed along the line");
    if !said.is_empty() {
        message += &format!(", {}", said.join(", "));
    }
    message += ". Apply settings reads the line again after it changes";
    Ok((line, front, message))
}

/// Whether a picked line's sketch (`document.inspect`'s `sketches`) or body (`body_names`) is
/// still in the design.
fn there(pick: &Value, design: &Value, bodies: &Value) -> Result<(), String> {
    let found = match (pick["sketch"].as_u64(), pick["body"].as_str()) {
        (Some(id), _) => (design["sketches"].as_array().into_iter().flatten())
            .any(|s| s["id"].as_u64() == Some(id)),
        (None, Some(body)) => {
            (bodies["bodies"].as_array().into_iter().flatten()).any(|b| b["name"] == body)
        }
        (None, None) => true, // what it is, `along_line` says
    };
    check(found, "the picked line is no longer in the design")
}

/// A sketch line's driving dimension as the parameter to follow: named after the sketch and
/// the line first when the app named it (`d1`). None without one.
fn follow(
    dimension: Option<String>,
    sketch: Option<(Value, String)>,
    design: &Value,
    run: Run,
) -> Result<Option<String>, String> {
    let (Some(dimension), Some((sketch, curve))) = (dimension, sketch) else {
        return Ok(None);
    };
    if !line::unnamed(&dimension) {
        return Ok(Some(dimension));
    }
    let sketch_name = sketch["name"].as_str().unwrap_or("Line");
    let name = line::dimension_name(sketch_name, &curve, design);
    let rename = json!({"name": dimension, "new_name": name});
    run("parameters.rename", rename)?;
    Ok(Some(name))
}

/// Place Along a line's fields, which only a cabinet placed so keeps.
const LINE_FIELDS: [&str; 6] = [
    "line",
    "height_line",
    "start_at",
    "offset",
    "faces",
    "width_from_line",
];

/// Whether a cabinet's params place it along a line it remembers.
fn along(params: &Value) -> bool {
    params["line"].as_array().is_some_and(|l| !l.is_empty())
}

/// Grounds a cabinet made on the origin and moves it along its line: turned to face the
/// front, its translation expressions over its own width and depth (`Kit`'s `centre`), so it
/// grows away from its Start at end. A grounded occurrence keeps following them only when it
/// was grounded before the move (grounded after, it stays where it was).
fn place_along(
    occurrence: &Value,
    summary: &mut Value,
    params: &Value,
    found: &(line::Line, [f64; 2], String),
    run: Run,
) -> Result<(), String> {
    let (line, front, message) = found;
    let start = params["start_at"].as_str().unwrap_or("left");
    let offset = num(params, "offset", 0.0)? * unit_mm(params);
    let centre = &summary["centre"];
    let size = [0, 1].map(|i| centre[i].as_str().unwrap_or("0 mm").to_string());
    let (at, degrees) = line::translate(line, *front, start, offset, [&size[0], &size[1]]);
    run(
        "occurrence.ground",
        json!({"occurrence": occurrence, "grounded": true}),
    )?;
    if degrees != 0 {
        run(
            "occurrence.move",
            json!({"occurrence": occurrence, "axis": [0, 0, 1],
                "angle": format!("{degrees} deg"), "move_grounded": true}),
        )?;
    }
    run(
        "occurrence.move",
        json!({"occurrence": occurrence, "space": "world", "translate": at, "move_grounded": true}),
    )?;
    summary["placed"] = json!("along the line");
    summary["line"] = json!(message);
    Ok(())
}

fn entries(entry: &str, params: &Value, stored: &Value, run: Run) -> Result<Value, String> {
    // One Cabinet dialog for both builders: a corner shape goes to the corner one, with the
    // fields it knows (its `doors` are the dialog's `corner_doors`), and is remembered as such.
    let corner;
    let (entry, params) = match params["shape"].as_str() {
        Some("L-shaped" | "diagonal") if entry == "cabinet" => {
            let mut p = params.clone();
            if let Some(all) = p.as_object_mut() {
                if let Some(doors) = all.remove("corner_doors") {
                    all.insert("doors".into(), doors);
                }
            }
            corner = p;
            ("corner", &corner)
        }
        _ => (entry, params),
    };
    match entry {
        "cabinet" | "corner" => {
            // Where it goes: on the origin, or against the cabinet that is selected.
            let mut place = params["place"].as_str().unwrap_or("origin");
            check(
                matches!(
                    place,
                    "origin" | "auto" | "right" | "left" | "above" | "line"
                ),
                "place must be origin, auto, right, left, above or line",
            )?;
            let matching = match params["match"].as_str().unwrap_or("no") {
                "no" => false,
                "selected" => true,
                _ => return Err("match must be no or selected".into()),
            };
            let mut params = params.clone();
            settings::design(entry, &mut params, stored);
            // The settings' defaults for its per-cabinet fields now, which it remembers: Apply
            // settings later changes only those it still has (`settings::apply`).
            params["_defaults"] = settings::defaults(entry, &params["units"], stored);
            let design = run(
                "document.inspect",
                json!({"bodies": false, "timeline": false}),
            )?;
            let picked = select::picked(&params, &design);
            // Match selected with nothing picked or selected: the dialog hides the field then,
            // but still sends a value it had.
            let matching = matching && !picked.is_empty();
            let measure = match matching || !matches!(place, "origin" | "line") {
                true => picked_measure(&picked, run)?,
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
            let (mut beside, mut beyond) = (None, false);
            // A name that is taken gets the next free number: Cabinet, Cabinet 2, Cabinet 3.
            let name = params["name"].as_str().unwrap_or("Cabinet");
            params["name"] = json!(kit::free_name(name, &design));
            // Against the selected cabinet: Auto settled above.
            let beside_one = !matches!(place, "origin" | "line");
            // Not along a line: no line fields, or Apply settings would place it along one.
            if place != "line" {
                if let Some(all) = params.as_object_mut() {
                    for key in LINE_FIELDS {
                        all.remove(key);
                    }
                }
            }
            // Along a line: read it now, as Width from line sizes the cabinet.
            let along = match place {
                "line" => Some(along_line(&mut params, &design, run)?),
                _ => None,
            };
            if matching || beside_one {
                let mut found = select::cabinet(&picked, &measure)?;
                if matching {
                    params["_match"] = kit::matched(&found.name, &design)?;
                    // the run's shared parameters, made and pointed at the first time
                    let setup = params["_match"]["setup"].take();
                    for step in setup.as_array().into_iter().flatten() {
                        run(step[0].as_str().unwrap_or_default(), step[1].clone())?;
                    }
                }
                // A panel on that side: the new cabinet stands beyond it, not linked, since the
                // link holds the two sides' seats together.
                beyond = matches!(place, "left" | "right") && found.with_panel(&measure, place);
                if beside_one {
                    beside = Some(found);
                }
            }
            if entry == "cabinet" {
                cabinet::from_drawers(&mut params)?;
            }
            let (mut summary, made) = make(entry, &params, &design, run)?;
            let mut held = false;
            if let Some(found) = &along {
                place_along(&made["occurrence"], &mut summary, &params, found, run)?;
                held = true; // grounded before the move, so it follows its sizes
            }
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
            // The picked cabinet, built again where it stands with the dialog's fields, which
            // start as it is: left so, nothing is built. Its parameters stay, so what is linked
            // to it still follows.
            let design = run(
                "document.inspect",
                json!({"bodies": false, "timeline": false}),
            )?;
            let picked = select::picked(params, &design);
            let measure = picked_measure(&picked, run)?;
            let mut names: Vec<String> = Vec::new();
            for body in &picked {
                if let Some(owner) = select::owner(&measure, body).map(String::from) {
                    if !names.contains(&owner) {
                        names.push(owner);
                    }
                }
            }
            let name = match names.as_slice() {
                [] => select::piece(&picked, &measure)?, // says why
                [one] => one.clone(),
                _ => return Err("Edit changes one cabinet: pick a board of only one".into()),
            };
            let about = run("component.metadata", json!({"component": name}))?;
            let ours = &about["metadata"][edit::KEY];
            let entry = ours["entry"].as_str().unwrap_or_default();
            // A cabinet made before Edit's file: the dialog could not show its values, so a
            // field still at the dialog's default is taken as unchanged (`edit::unshown`), and
            // the file is written now, for the next time.
            let file = run("_get_blob", json!({"name": edit::blob(&name)}));
            let unshown = file.as_ref().map_or(true, Value::is_null);
            edit::design_wide(entry, params).map_err(|e| format!("`{name}`: {e}"))?;
            let mut changes = edit::taken(entry, params);
            if unshown {
                changes = edit::unshown(&changes);
                let (_, now) = edit::merge(ours, &json!({}), &design)?;
                let parts = select::parts(&measure, &name);
                let copy = edit::copy(entry, &now, &name, parts.iter().map(|b| b.as_str()));
                let file = json!({"name": edit::blob(&name), "data": copy});
                run("_put_blob", file)?;
            }
            let changes = settings::edit_plinth(&changes, ours, &design, stored);
            let (entry, params) =
                edit::merge(ours, &changes, &design).map_err(|e| format!("`{name}`: {e}"))?;
            // The dialog shows the cabinet as it is: left as shown, nothing to build.
            let (_, base) = edit::merge(ours, &json!({}), &design)?;
            let changed = edit::changed(&params, &base);
            if changed.is_empty() {
                let message = format!("{name}: nothing changed");
                return Ok(json!({"name": name, "edited": false, "message": message}));
            }
            // A width or height typed in Edit is the cabinet's own from now on, not the line's.
            let mut params = params;
            if changed.iter().any(|c| c == "width") {
                params["width_from_line"] = json!(false);
            }
            if changed.iter().any(|c| c == "height") {
                params["height_line"] = json!([]);
            }
            let mut summary = rebuild(&name, &about, &entry, &params, &design, &measure, run)?;
            summary["edited"] = json!(true);
            summary["changed"] = json!(changed);
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
                    let names = body_names(run)?;
                    select::parts(&names, &select::piece(&picked, &names)?)
                }
                _ => return Err("scope must be cabinet or parts".into()),
            };
            // a glass door's pane stays glass
            let bodies: Vec<String> = bodies
                .into_iter()
                .filter(|b| !fittings::is_glass(b))
                .collect();
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
            let picked = select::picked(params, &design);
            let measure = picked_measure(&picked, run)?;
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
        "base" => {
            // A mobile base (#101) under the picked cabinets: a frame on casters in a component
            // of its own, at world coordinates like a worktop, and the cabinets stood on it.
            // Run again with its name, it is made again (over the cabinets it remembers when
            // none is picked), and the cabinets go up or down to its new top.
            let design = run(
                "document.inspect",
                json!({"bodies": false, "timeline": false}),
            )?;
            let names = body_names(run)?;
            let tree = run("component.list", json!({}))?;
            let components: Vec<&str> = (tree["components"].as_array().into_iter().flatten())
                .filter_map(|c| c["name"].as_str())
                .collect();
            let mut params = params.clone();
            let name = params["name"].as_str().unwrap_or("Base").trim().to_string();
            let mut cabinets: Vec<String> = Vec::new();
            for body in select::picked(&params, &design) {
                let cabinet = select::owner(&names, &body)
                    .ok_or(format!("`{body}` is not part of a cabinet"))?;
                if !cabinets.iter().any(|c| c == cabinet) {
                    cabinets.push(cabinet.to_string());
                }
            }
            let again = components.contains(&name.as_str());
            if again {
                let about = run("component.metadata", json!({"component": name}))?;
                let ours = &about["metadata"][edit::KEY];
                check(
                    ours["entry"] == "base",
                    &format!("name: `{name}` is taken; choose another"),
                )?;
                if cabinets.is_empty() {
                    let all = ours["cabinets"].as_array().into_iter().flatten();
                    cabinets = all.filter_map(|c| c.as_str().map(String::from)).collect();
                }
            }
            let measure = measured(names, &cabinets, run)?;
            // a cabinet deleted since the base remembered it is left out
            cabinets.retain(|c| select::named(&measure, c).is_ok());
            let (plans, floors) = base::plans(&measure, &cabinets)?;
            params["name"] = json!(name);
            let built = base::build(&params, &plans)?;
            let lift = built.1["lift"].as_f64().unwrap_or(0.0);
            if again {
                run("component.delete", json!({"component": name}))?;
            }
            // ponytail: made in the active component at world coordinates, as a worktop is, so
            // that component must stand on the origin, unturned.
            let (mut summary, made) = component(built, &params, "Birch plywood", run)?;
            run(
                "occurrence.ground",
                json!({"occurrence": made["occurrence"], "grounded": true}),
            )?;
            // The cabinets on the frame: each grounded one moved up by its own height to go; one
            // held to its neighbour goes with it.
            let mut raised = Map::new();
            for (cabinet, floor) in cabinets.iter().zip(&floors) {
                // to the micrometre: a board with holes measures a hair off
                let up = ((lift - floor) * 1e3).round() / 1e3;
                let (_, _, grounded) = edit::placement(&tree, cabinet)?;
                if grounded && up.abs() > 1e-6 {
                    run(
                        "occurrence.move",
                        json!({"occurrence": format!("{cabinet}:1"), "space": "world",
                            "translate": [0, 0, up], "move_grounded": true}),
                    )?;
                }
                raised.insert(cabinet.clone(), json!(up));
            }
            // Each must stand on the frame now: one held to a cabinet that is not on it (or
            // not grounded and held by nothing) did not move, and the run is refused.
            let now = measured(body_names(run)?, &cabinets, run)?;
            let (_, after) = base::plans(&now, &cabinets)?;
            for (cabinet, floor) in cabinets.iter().zip(after) {
                check(
                    (floor - lift).abs() < 0.5,
                    &format!("`{cabinet}` did not go up onto the base: it is held to a cabinet that is not on it, or stands loose; pick that cabinet too, or ground `{cabinet}`"),
                )?;
            }
            // Connector bolts through the sides that touch (#198), each cut in its cabinet's own
            // component; a hole already there (a run again) is left as it is.
            // ponytail: a cut the cabinet does not remember: Edit cabinet makes its side whole
            // again (run Mobile base again), and a cabinet moved away keeps its old holes.
            let timeline = run(
                "document.inspect",
                json!({"bodies": false, "timeline": "summary"}),
            )?;
            let had: Vec<&str> = (timeline["timeline"].as_array().into_iter().flatten())
                .filter_map(|f| f["name"].as_str())
                .collect();
            let tree = run("component.list", json!({}))?;
            let mut cuts: Vec<(String, Commands)> = Vec::new();
            let holes = base::bolts(&now, &cabinets);
            // the frame's boards beside the cabinets, for the screws into it
            let mut both = now.clone();
            let frame = run("inspect.measure", json!({"bodies": summary["bodies"]}))?;
            if let (Some(all), Some(boards)) =
                (both["bodies"].as_array_mut(), frame["bodies"].as_array())
            {
                all.extend(boards.iter().cloned());
            }
            let screws = base::screws(&both, &cabinets, &name);
            // each cut: its feature's name, the board it goes into, its box in the world, its size
            let mut wanted: Vec<(String, String, [[f64; 3]; 2], f64)> = Vec::new();
            for (sides, n, world) in &holes {
                for (k, (side, what)) in sides.iter().zip(["bolt", "hole"]).enumerate() {
                    let other = &sides[1 - k];
                    let feature = format!("{side} connector {what} {n} to {other}");
                    wanted.push((feature, side.clone(), *world, base::BOLT_D));
                }
            }
            // A screw's name holds where it is, so a frame made again with other sizes finds its
            // old holes stale: they are deleted, and the new ones cut.
            for screw in &screws {
                let (bottom, _, _, world) = screw;
                wanted.push((
                    base::screw_name(screw),
                    bottom.clone(),
                    *world,
                    base::SCREW_D,
                ));
            }
            let names: Vec<String> = wanted.iter().map(|w| w.0.clone()).collect();
            for cabinet in &cabinets {
                let stale = base::stale_screws(&had, &names, cabinet);
                if !stale.is_empty() {
                    cuts.push((
                        cabinet.clone(),
                        vec![("timeline.delete", json!({"features": stale}))],
                    ));
                }
            }
            for (feature, side, world, d) in &wanted {
                let cabinet = select::owner(&now, side).unwrap_or_default().to_string();
                if had.contains(&feature.as_str()) || cabinet.is_empty() {
                    continue;
                }
                let (corner, size) = worktop::local(*world, &edit::placement(&tree, &cabinet)?.1);
                let along = (0..3)
                    .max_by(|&a, &b| size[a].total_cmp(&size[b]))
                    .unwrap_or(0);
                let base =
                    [0, 1, 2].map(|i| corner[i] + if i == along { 0.0 } else { size[i] / 2.0 });
                let mut axis = [0, 0, 0];
                axis[along] = 1;
                let args = json!({
                    "base": base.map(kit::mm), "axis": axis, "diameter": kit::mm(*d),
                    "height": kit::mm(size[along]), "operation": "cut", "targets": [side],
                    "name": feature,
                });
                match cuts.iter_mut().find(|c| c.0 == cabinet) {
                    Some(c) => c.1.push(("solid.cylinder", args)),
                    None => cuts.push((cabinet, vec![("solid.cylinder", args)])),
                }
            }
            for (cabinet, commands) in cuts {
                inside(&cabinet, commands, run)?;
            }
            summary["bolts"] = json!(holes.len());
            summary["screws"] = json!(screws.len());
            let mut values = edit::remember("base", &params, &name);
            values[edit::KEY]["cabinets"] = json!(cabinets);
            run(
                "component.metadata.set",
                json!({"component": name, "values": values}),
            )?;
            summary["cabinets"] = json!(cabinets);
            summary["raised"] = Value::Object(raised);
            Ok(summary)
        }
        "worktop" => {
            // A worktop over the selected cabinets, in a component of its own on the origin.
            // Run again it replaces the worktop of that name, and any worktop picked.
            let design = run(
                "document.inspect",
                json!({"bodies": false, "timeline": false}),
            )?;
            let names = body_names(run)?;
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
                if let Some(cabinet) = select::owner(&names, &body) {
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
            // Only the cabinets under it are measured. A cabinet deleted since a worktop
            // remembered it is left out.
            let measure = measured(names, &cabinets, run)?;
            cabinets.retain(|c| select::named(&measure, c).is_ok());
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
            // A corner cabinet's L boards fill no box, so they are among these: their open
            // corners from the same mesh (`_corners`, drawn in the layouts).
            let (l_boards, mut corners) = (cutlist::l_boards(&design), serde_json::Map::new());
            for body in cutlist::turned(&measure) {
                let mesh = run("model.mesh", json!({"body": body, "tolerance": 1.0}))?;
                if let Some(corner) =
                    cutlist::open_corner(&mesh).filter(|_| l_boards.contains(&body))
                {
                    corners.insert(body.clone(), json!(corner));
                }
                local.insert(body, cutlist::extents(&mesh));
            }
            let mut params = params.clone();
            settings::design(entry, &mut params, stored);
            params["_notched"] = json!(cutlist::notched(&design));
            params["_grooved"] = json!(cutlist::grooved(&design));
            params["_fingered"] = json!(cutlist::fingered(&design));
            params["_corners"] = Value::Object(corners);
            let params = &params;
            let mut list = cutlist::cutlist(&measure, &local, imperial, params)?;
            list["hardware"] = cutlist::hardware(&measure, &local, &design);
            // Prices from Woodwork settings: the estimate, and what has no price yet (#56).
            let prices = params["prices"].as_str().unwrap_or_default();
            list["cost"] = cost::estimate(&list, prices)?;
            if !prices.trim().is_empty() {
                let csv = cost::csv(&list["cost"]);
                if let Some(text) = list["csv"].as_str().map(|t| t.to_string() + &csv) {
                    list["csv"] = json!(text);
                }
            }
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
            measure["grooved"] = json!(cutlist::grooved(&design));
            measure["fingered"] = json!(cutlist::fingered(&design));
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
        // the entry for scripts, and one with a dialog per section: the same storage
        "settings" | "settings-general" | "settings-fronts" | "settings-drawers"
        | "settings-plinth" | "settings-joinery" | "settings-cutlist" | "settings-display" => {
            let stored = run("storage.get", json!({}))?;
            run(
                "storage.patch",
                json!({"patch": settings::patch(params, &stored)?}),
            )?;
            let stored = run("storage.get", json!({}))?;
            // See-through boards: our materials made so, or solid again, now.
            let changed = material::see_through(run, material::opacity(&stored))?;
            // Settings only stores them: Apply settings builds cabinets again with them.
            Ok(json!({"settings": stored["document"]["settings"], "see_through": changed}))
        }
        // Apply settings (a job, 0.48): the cabinets picked in its field, or every one when it
        // is empty (not the design's selection, which a script may have left), built again with
        // the settings or as each was made, with progress per cabinet and no 30 s limit.
        "rebuild" => {
            let stored = run("storage.get", json!({}))?;
            let all = params["selection"].as_array().is_none_or(|p| p.is_empty());
            apply(params, &stored, all, params["with"] != "made", run)
        }
        _ => Err(format!("unknown entry `{entry}`")),
    }
}

/// What the build would refuse in the Cabinet dialog's own fields (`values`), with the
/// design-wide settings of `storage.get`'s reply filled in as the run does, as the field it is
/// about and the reason: the build's checks (`cabinet::checked`, `corner::checked`), no design
/// read. A corner shape is the corner builder's, its doors `corner_doors`.
fn form_problem(values: &Value, stored: &Value) -> Option<(&'static str, String)> {
    let corner = matches!(values["shape"].as_str(), Some("L-shaped" | "diagonal"));
    let mut p = values.clone();
    if let (true, Some(all)) = (corner, p.as_object_mut()) {
        if let Some(doors) = all.remove("corner_doors") {
            all.insert("doors".into(), doors);
        }
    }
    settings::design(if corner { "corner" } else { "cabinet" }, &mut p, stored);
    let checked = match corner {
        true => corner::checked(&p),
        false => cabinet::checked(&p),
    };
    let (field, why) = settings::problem(&checked.err()?, corner);
    Some((settings::on_cabinet(field), why))
}

/// The dialog hook (`dialog@0.1`): the Cabinet and Corner dialogs' defaults from Woodwork
/// settings. It runs outside a plugin run, so `storage.get` names our plugin. Edit's: the
/// fields the selected cabinet does not take, hidden.
pub fn dialog(entry: &str, values: &Value, run: Run) -> Result<Value, String> {
    // Mode Auto with a door, front, shelf... picked: Edit picked's fields, the new cabinet's
    // hidden (`select::auto`, as the run decides).
    let mut read = None;
    if entry == "cabinet" && values["mode"] == "auto" {
        let design = run(
            "document.inspect",
            json!({"bodies": "names", "timeline": false}),
        );
        let picked = (design.as_ref()).map_or(vec![], |d| select::picked(values, d));
        let auto = (design.as_ref()).map_or(select::Auto::New, |d| select::auto(d, &picked));
        if let select::Auto::Edit(..) = auto {
            let mut edit = values.clone();
            edit["mode"] = json!("edit");
            let mut reply = dialog("cabinet", &edit, run)?;
            for key in edit::NEW_ONLY {
                reply[key] = json!({"visible": false});
            }
            reply["mode"] = json!({ "hint": auto.hint() });
            return Ok(reply);
        }
        read = Some(design.clone()); // read once: the new cabinet's fields below use it too
    }
    if entry == "cabinet" && values["mode"] == "edit" {
        let mut reply = dialog("edit", values, run)?;
        // Cabinet's Shape: a box stays a box (greyed), a corner a corner
        if reply["corner_doors"]["visible"] == false {
            reply["shape"] = json!({"default": "box", "enabled": false});
        }
        return Ok(reply);
    }
    if entry == "edit" {
        // Kept light, as the hook has 10M fuel and reading a 30-cabinet design's body names and
        // parameters takes most of it: the picked boards from the dialog's field (the design's
        // selection only when it is empty, as for a script), the cabinets from the names of our
        // files (`storage.get`, small), and each picked cabinet's file (#128). A failing hook
        // fails `plugin.run`, so a read that fails hides nothing.
        let field = select::picked(values, &Value::Null);
        let read = |run: Run| {
            run(
                "document.inspect",
                json!({"bodies": "names", "timeline": false}),
            )
        };
        // the design's selection alone (`_selection`)
        let picked = match field.is_empty() {
            true => run("_selection", json!({}))
                .map(|d| select::picked(values, &d))
                .unwrap_or_default(),
            false => field,
        };
        let stored = run("storage.get", json!({"plugin": "woodwork"})).unwrap_or_default();
        let files: Vec<&str> = (stored["blobs"].as_array().into_iter().flatten())
            .filter_map(|b| {
                b["name"]
                    .as_str()?
                    .strip_prefix("edit/")?
                    .strip_suffix(".json")
            })
            .collect();
        // A board's cabinet: the longest of our files' names its name starts with, whose file
        // lists the board's first word (names alone would give "Cabinet 2"'s boards to "Cabinet").
        let Some(body) = picked.first() else {
            return Ok(json!({}));
        };
        let prefix = kit::prefix(body);
        let mut owner: Option<(&str, Value)> = None;
        for &f in &files {
            let longer = owner.as_ref().is_none_or(|(o, _)| f.len() > o.len());
            if !longer || !prefix.strip_prefix(f).is_some_and(|r| r.starts_with('_')) {
                continue;
            }
            let blob = run("_get_blob", json!({"name": format!("edit/{f}.json")}));
            let blob = blob.unwrap_or_default();
            // a file from before the boards' names were kept has no `parts`
            let lists = blob.get("parts").is_none_or(|parts| {
                let word = prefix[f.len() + 1..].split('_').next();
                (parts.as_array().into_iter().flatten()).any(|p| p.as_str() == word)
            });
            if lists {
                owner = Some((f, blob));
            }
        }
        if let Some((f, ours)) = owner {
            if let (Some(entry), Some(shown)) = (ours["entry"].as_str(), ours["shown"].as_object())
            {
                // its sizes as they are in the design now, in the cabinet's units
                let names: Vec<String> = (edit::SHOWN_SIZES.iter())
                    .map(|k| format!("{f}_{k}"))
                    .collect();
                let found = run("_params", json!({ "names": names })).unwrap_or_default();
                let scale = if ours["units"] == "in" { 25.4 } else { 1.0 };
                let mut sizes = Map::new();
                for (key, name) in edit::SHOWN_SIZES.iter().zip(&names) {
                    if let Some(mm) = found[name].as_f64() {
                        sizes.insert(key.to_string(), json!((mm / scale * 1e4).round() / 1e4));
                    }
                }
                let mut reply = edit::fields(entry, shown, &sizes);
                // A change the build would refuse, said on its field (as for a new cabinet);
                // only when the cabinet as it is passes, so a field the change does not touch is
                // never blamed.
                let mut now = shown.clone();
                now.extend(sizes);
                now.insert("name".into(), json!(f));
                now.insert("units".into(), ours["units"].clone());
                let remembered = json!({"entry": entry, "params": now});
                let checked = |changes: &Value| {
                    let (entry, p) = edit::merge(&remembered, changes, &json!({}))?;
                    match entry.as_str() {
                        "corner" => corner::checked(&p),
                        _ => cabinet::checked(&p),
                    }
                };
                let changes = edit::unshown(&edit::taken(entry, values));
                if let (Ok(()), Err(why)) = (checked(&json!({})), checked(&changes)) {
                    let (field, why) = settings::problem(&why, entry == "corner");
                    let text = format!("{why}: nothing is changed or previewed until this is");
                    reply[settings::on_cabinet(field)]["hint"] = json!(text);
                }
                return Ok(reply);
            }
        }
        // A cabinet made before has no file: only the other kind's fields are hidden, and the
        // run then writes its file (nothing built).
        let Ok(design) = read(run) else {
            return Ok(json!({}));
        };
        return Ok(edit::hidden(values, &design));
    }
    let stored = run("storage.get", json!({"plugin": "woodwork"}))?;
    // Apply settings: which cabinets the picked parts are, from our files' names (`storage.get`,
    // small: no design read for the 10M fuel), as the run resolves them.
    if entry == "rebuild" {
        let files: Vec<String> = (stored["blobs"].as_array().into_iter().flatten())
            .filter_map(|b| {
                b["name"]
                    .as_str()?
                    .strip_prefix("edit/")?
                    .strip_suffix(".json")
            })
            .map(String::from)
            .collect();
        let picked = select::picked(values, &Value::Null);
        return Ok(json!({"selection": {"hint": select::applies_to(&files, &picked)}}));
    }
    let mut reply = settings::fields(entry, values, &stored);
    // A combination the build refuses (Doors 3 without dividers, ...), said at once on its
    // field, as the preview then shows nothing: the build's own checks of the fields, with the
    // design-wide settings filled in as the run does (no design read).
    if entry == "cabinet" {
        if let Some((field, why)) = form_problem(values, &stored) {
            reply[field]["hint"] = json!(format!(
                "{why}: nothing is made or previewed until this is changed"
            ));
        }
    }
    // Woodwork settings: a size out of bounds, or settings a new cabinet would refuse, said on
    // the field of this dialog it is about (another section's is not shown here).
    if entry == "settings" || entry.starts_with("settings-") {
        if let Some((field, why)) = settings::trouble(values, &stored) {
            if let Some(patch) = reply.get_mut(&field).filter(|f| f.is_object()) {
                patch["hint"] = json!(format!("{why}: new cabinets would be refused"));
            }
        }
    }
    // Match selected: Yes when a cabinet is picked, hidden when none is. A second read, a few
    // ms on the sample kitchen; if it fails, the field stays as it is.
    if entry == "cabinet" {
        let design = read.unwrap_or_else(|| {
            run(
                "document.inspect",
                json!({"bodies": "names", "timeline": false}),
            )
        });
        if let Ok(design) = design {
            reply["match"] = settings::matching(values, &design);
            if let Some(all) = reply.as_object_mut() {
                all.extend(settings::matched_sizes(values, &design));
            }
            // Mode Auto, a new cabinet (`select::auto`): the Edit fields hidden, and the
            // hint says so. (The hook cannot set Mode: a reply that does is refused whole.)
            if values["mode"] == "auto" {
                let picked = select::picked(values, &design);
                reply["mode"] = json!({"hint": select::auto(&design, &picked).hint()});
                for key in edit::EDIT_ONLY {
                    reply[key] = json!({"visible": false});
                }
            }
        }
    }
    Ok(reply)
}

#[cfg(test)]
mod tests;

#[cfg(target_arch = "wasm32")]
mod guest {
    wit_bindgen::generate!({ path: "wit", world: "woodwork" });

    use craft::plugin::types::{FieldFailure, PluginError};

    struct Woodwork;

    fn invalid(reason: String) -> PluginError {
        PluginError::InvalidInput(FieldFailure {
            field: "params".into(),
            reason,
        })
    }

    fn run(command: &str, args: serde_json::Value) -> Result<serde_json::Value, String> {
        // Our files in the design (`storage@0.1`): JSON, a missing file read as null.
        if command == "_put_blob" {
            let (name, data) = (
                args["name"].as_str().unwrap_or_default(),
                args["data"].to_string(),
            );
            craft::plugin::storage::put_blob(name, data.as_bytes(), false, false)
                .map_err(|e| format!("{command}: {e:?}"))?;
            return Ok(serde_json::Value::Null);
        }
        if command == "_get_blob" {
            let name = args["name"].as_str().unwrap_or_default();
            let bytes = craft::plugin::storage::get_blob(Some("woodwork"), name, 0, 16 * 1024)
                .map_err(|e| format!("{command}: {e:?}"))?;
            return Ok(serde_json::from_slice(&bytes).unwrap_or_default());
        }
        // The design's selection, from `document.inspect`, whose other keys are not parsed: they
        // are every parameter (78 KB in a 30-cabinet design), costly for the 10M fuel of a hook.
        if command == "_selection" {
            let args = serde_json::json!({"bodies": false, "timeline": false});
            let reply = craft::plugin::commands::run("document.inspect", &args.to_string())
                .map_err(|e| format!("{command}: {e:?}"))?;
            let at = reply.rfind("\"selection\"").ok_or("no selection")?;
            let rest = reply[at + 11..].trim_start().trim_start_matches(':');
            let mut values = serde_json::Deserializer::from_str(rest).into_iter();
            let selection: serde_json::Value = values
                .next()
                .ok_or("no selection")?
                .map_err(|e| e.to_string())?;
            return Ok(serde_json::json!({ "selection": selection }));
        }
        // Some parameters' values (mm) by name, from `document.inspect`'s reply without parsing it.
        if command == "_params" {
            let args2 = serde_json::json!({"bodies": false, "timeline": false});
            let reply = craft::plugin::commands::run("document.inspect", &args2.to_string())
                .map_err(|e| format!("{command}: {e:?}"))?;
            let mut found = serde_json::Map::new();
            for name in args["names"].as_array().into_iter().flatten() {
                let Some(name) = name.as_str() else { continue };
                if let Some(v) = crate::kit::param_value(&reply, name) {
                    found.insert(name.into(), v.into());
                }
            }
            return Ok(serde_json::Value::Object(found));
        }
        // A job's step (`apply`): the operations panel's bar and name.
        if command == "_progress" {
            let fraction = args["fraction"].as_f64().unwrap_or(0.0) as f32;
            let message = args["message"].as_str().unwrap_or_default();
            craft::plugin::control::progress(fraction, message);
            return Ok(serde_json::Value::Null);
        }
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

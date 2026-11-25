use super::*;

/// The commands a run of `entry` calls, against an app that answers every command with `{}`.
fn calls(entry: &str) -> Vec<String> {
    let mut called = Vec::new();
    let mut run = |command: &str, _: Value| {
        called.push(command.to_string());
        Ok(json!({}))
    };
    let _ = execute(entry, &json!({}), &mut run);
    called
}

#[test]
fn a_run_that_builds_a_cabinet_frames_the_model() {
    for entry in ["cabinet", "corner"] {
        let called = calls(entry);
        // Fit frames the selection, so it is cleared first.
        assert_eq!(
            called[called.len() - 2..],
            ["select.clear", "view.fit"],
            "{entry}"
        );
        assert_eq!(called.iter().filter(|c| *c == "view.fit").count(), 1);
    }
    // Camera `leave` in Woodwork settings: cleared, not framed.
    let mut called = Vec::new();
    let mut run = |command: &str, _: Value| {
        called.push(command.to_string());
        Ok(json!({"document": {"settings": {"camera": "leave"}}}))
    };
    let _ = execute("cabinet", &json!({}), &mut run);
    // read once, first: it also holds the design-wide joinery and hardware
    assert_eq!(called[0], "storage.get");
    assert_eq!(called.last().unwrap(), "select.clear");
    // Material and the cut list change nothing worth framing.
    assert!(!calls("material").contains(&"view.fit".to_string()));
    assert!(!calls("cutlist").contains(&"view.fit".to_string()));
}

#[test]
fn drawing_fills_in_ids_and_replaces_the_old_sheet() {
    let side = |name: &str, x: f64| {
        json!({"name": name, "volume_mm3": 18.0 * 560.0 * 720.0,
        "bbox": {"min": [x, -280, 0], "max": [x + 18.0, 280, 720]}})
    };
    let run_with = |drawings: Value, noted: Value| {
        let mut called: Vec<(String, Value)> = Vec::new();
        let mut views = 0;
        let mut run = |command: &str, args: Value| {
            called.push((command.to_string(), args));
            Ok(match command {
                "inspect.measure" => {
                    json!({"bodies": [side("Base Side L", -300.0), side("Base Side R", 282.0)]})
                }
                "component.list" => json!({"components": [{"id": 0, "occurrences": []},
                    {"id": 3, "name": "Base", "occurrences": [{"id": 4, "parent": 0}]}]}),
                "drawing.list" => drawings.clone(),
                "drawing.open" | "drawing.create" => json!({"sources": [{"id": "src"}],
                    "sheets": [{"id": "old", "name": "Base", "title_block": {"cells": [{"name": "mass"}]}}],
                    "properties": {drawing::KEY: noted}}),
                "drawing.sheet.add" => json!({"sheet": "new", "sheets": []}),
                "drawing.view.base" | "drawing.view.projected" => {
                    views += 1;
                    json!({"view": format!("v{views}"), "sheet": "new"})
                }
                _ => json!({}),
            })
        };
        let reply = execute("drawing", &json!({"scope": "all"}), &mut run);
        (called, reply)
    };
    let woodwork = json!({"drawings": [{"id": "d1", "name": "Woodwork"}]});
    let (called, reply) = run_with(woodwork.clone(), json!({"old": "0", "deleted": "1"}));
    assert_eq!(
        reply.unwrap(),
        json!({"drawing": "Woodwork", "sheets": ["Base"], "remaining": [], "message": "Drew 1 sheet"})
    );
    let find = |command: &str| called.iter().find(|c| c.0 == command).unwrap().1.clone();
    assert_eq!(find("drawing.open"), json!({"drawing": "d1"}));
    // in mm, whatever the design's units or what the drawing was set to
    assert_eq!(find("drawing.set"), json!({"units": "mm"}));
    assert_eq!(
        find("drawing.sheet.add")["documents"],
        json!({"source": "src", "component": 3})
    );
    // its title block without the cells that weigh the cabinet
    assert_eq!(
        find("drawing.sheet.add")["title_block"],
        json!({"cells": []})
    );
    assert_eq!(find("drawing.view.base")["sheet"], "new");
    assert_eq!(find("drawing.view.projected")["parent"], "v1");
    let right = called
        .iter()
        .rfind(|c| c.0 == "drawing.dimension.add")
        .unwrap();
    assert_eq!(right.1["view"], "v2");
    // the old sheet goes once the new one is drawn, the new one's fingerprint is noted (the
    // old sheet's dropped), and the drawing is closed
    let names: Vec<&str> = called.iter().map(|c| c.0.as_str()).collect();
    assert_eq!(
        names[names.len() - 4..],
        [
            "drawing.note.add",
            "drawing.sheet.delete",
            "drawing.properties.set",
            "drawing.close"
        ]
    );
    assert_eq!(
        called[names.len() - 3].1,
        json!({"sheet": "old", "contents": "delete"})
    );
    let noted = find("drawing.properties.set");
    assert_eq!(noted["key"], drawing::KEY);
    let print = noted["value"]["new"].clone();
    assert_eq!(noted["value"], json!({"new": print}));
    // Drawn again with that fingerprint on its sheet: up to date, nothing drawn.
    let (called, reply) = run_with(woodwork, json!({"old": print}));
    let reply = reply.unwrap();
    assert_eq!(reply["sheets"], json!([]));
    assert_eq!(reply["remaining"], json!([]));
    assert_eq!(reply["message"], "All sheets were up to date");
    assert!(!called.iter().any(|c| c.0 == "drawing.sheet.add"));
    let kept = called
        .iter()
        .find(|c| c.0 == "drawing.properties.set")
        .unwrap();
    assert_eq!(kept.1["value"], json!({"old": print}));
    // no drawing yet: one is made, and its empty first sheet deleted
    let (called, _) = run_with(json!({"drawings": []}), json!(null));
    let created = called.iter().find(|c| c.0 == "drawing.create").unwrap();
    assert_eq!(created.1["units"], "mm");
    assert!(!called.iter().any(|c| c.0 == "drawing.open"));
    // nothing to draw, or a scope we do not know
    assert_eq!(
        calls("drawing").last().map(String::as_str),
        Some("inspect.measure")
    );
    let mut run = |_: &str, _: Value| Ok(json!({}));
    let err = execute("drawing", &json!({"scope": "every"}), &mut run).unwrap_err();
    assert_eq!(err, "scope must be selected or all");
    let err = execute("drawing", &json!({"scope": "all"}), &mut run).unwrap_err();
    assert_eq!(err, "the design has no cabinets");
}

#[test]
fn drawing_every_cabinet_stops_before_the_command_limit() {
    // 25 cabinets of 11 commands a sheet: 22 fit after the 7 that read and open, and the 3
    // that end the run.
    let n = 25;
    let side = |name: String, x: f64| {
        json!({"name": name, "volume_mm3": 18.0 * 560.0 * 720.0,
        "bbox": {"min": [x, -280, 0], "max": [x + 18.0, 280, 720]}})
    };
    let bodies: Vec<Value> = (0..n)
        .flat_map(|i| {
            let x = i as f64 * 600.0;
            [
                side(format!("C{i} Side L"), x),
                side(format!("C{i} Side R"), x + 582.0),
            ]
        })
        .collect();
    let components: Vec<Value> = (0..n)
        .map(|i| json!({"id": i + 1, "name": format!("C{i}"), "occurrences": [{"id": 100 + i, "parent": 0}]}))
        .chain([json!({"id": 0, "occurrences": []})])
        .collect();
    let mut count = 0;
    let mut run = |command: &str, _: Value| {
        count += 1;
        Ok(match command {
            "inspect.measure" => json!({ "bodies": bodies }),
            "component.list" => json!({ "components": components }),
            "drawing.create" => json!({"sources": [{"id": "src"}], "sheets": []}),
            "drawing.sheet.add" => json!({"sheet": format!("s{count}")}),
            _ => json!({}),
        })
    };
    let reply = execute("drawing", &json!({"scope": "all"}), &mut run).unwrap();
    assert_eq!(reply["sheets"].as_array().unwrap().len(), 22);
    assert_eq!(reply["remaining"], json!(["C22", "C23", "C24"]));
    assert_eq!(
        reply["message"],
        "Drew 22 of 25 sheets; run Drawing again for the rest"
    );
    assert!(count <= 256, "{count}");
}

#[test]
fn a_panel_is_made_in_its_cabinet_and_kept() {
    let side = |name: &str, x: f64| {
        json!({"name": name, "material": "Oak", "volume_mm3": 18.0 * 560.0 * 720.0,
        "bbox": {"min": [x, -280, 0], "max": [x + 18.0, 280, 720]}})
    };
    // A on the origin, B held to its right side
    let run_with = |params: Value| {
        let mut called: Vec<(String, Value)> = Vec::new();
        let mut run = |command: &str, args: Value| {
            called.push((command.to_string(), args.clone()));
            Ok(match command {
                "document.inspect" => json!({"selection": [{"name": "A Side L"}],
                    "params": [{"name": "A_thickness", "value": 18.0}]}),
                "inspect.measure" => {
                    json!({"bodies": [side("A Side L", -300.0), side("A Side R", 282.0)]})
                }
                "component.list" => json!({"components": [
                    {"id": 1, "name": "A", "occurrences": [{"id": 2, "parent": 0}]},
                    {"id": 3, "name": "B", "occurrences": [{"id": 4, "parent": 0}]}]}),
                "joint.list" => json!({"joints": [{"name": "B link", "a": 2, "b": 4}]}),
                "component.metadata" if args["component"] == "B" => {
                    json!({"metadata": {edit::KEY: {"link": {"to": "A", "place": "right"}}}})
                }
                "component.metadata" => json!({"metadata": {edit::KEY: {"entry": "cabinet",
                    "params": {"name": "A"}, "panels": [{"kind": "end", "side": "left"}]}}}),
                _ => json!({}),
            })
        };
        let reply = execute("panel", &params, &mut run);
        (called, reply)
    };
    let (_, reply) = run_with(json!({"kind": "end", "side": "right"}));
    assert_eq!(
        reply.unwrap_err(),
        "`A` has a cabinet against its right: a panel there would stand in it"
    );
    // on Auto by the picked Side L: made inside A, in the Side L's material, and remembered
    // in place of an old one on that side whose boards were deleted
    let (called, reply) = run_with(json!({"kind": "end", "side": "auto"}));
    assert_eq!(
        reply.unwrap(),
        json!({"cabinet": "A", "side": "left", "bodies": ["A End panel L"]})
    );
    let names: Vec<&str> = called.iter().map(|c| c.0.as_str()).collect();
    let at = |c: &str| names.iter().position(|n| *n == c).unwrap();
    assert!(at("component.activate") < at("solid.box"));
    let find = |command: &str| called.iter().find(|c| c.0 == command).unwrap().1.clone();
    assert_eq!(find("material.assign")["material"], "Oak");
    let kept = &find("component.metadata.set")["values"][edit::KEY];
    assert_eq!(kept["panels"].as_array().unwrap().len(), 1);
    assert_eq!(kept["panels"][0]["kind"], "end");
    assert_eq!(kept["panels"][0]["units"], "mm");
    // a cabinet placed against a side that carries a panel stands beyond it, not linked
    let mut moved = json!(null);
    let mut run = |command: &str, args: Value| {
        if command == "occurrence.move" {
            moved = args;
        }
        Ok(match command {
            "document.inspect" => json!({"selection": [{"name": "A Side R"}]}),
            "inspect.measure" => json!({"bodies": [side("A Side L", -300.0),
                side("A Side R", 282.0), side("A End panel R", 300.0)]}),
            _ => json!({}),
        })
    };
    let reply = execute("cabinet", &json!({"place": "right"}), &mut run).unwrap();
    assert_eq!(
        (&reply["placed"], &reply["linked"]),
        (&json!("right of A"), &json!(false))
    );
    assert_eq!(moved["translate"][0], json!(318.0 + 300.0));
}

#[test]
fn settings_patch_then_show_and_the_hook_names_our_data() {
    assert_eq!(
        calls("settings"),
        [
            "storage.get",
            "storage.patch",
            "storage.get",
            "material.list"
        ]
    );
    let mut asked = Vec::new();
    let mut run = |command: &str, args: Value| {
        asked.push(json!([command, args]));
        Ok(json!({"document": {"settings": {"material": "MDF"}}}))
    };
    let reply = dialog("cabinet", &json!({}), &mut run).unwrap();
    // our data, then the body names for Match selected: two reads
    assert_eq!(asked[0], json!(["storage.get", {"plugin": "woodwork"}]));
    assert_eq!(asked[1][0], "document.inspect");
    assert_eq!(asked.len(), 2);
    assert_eq!(reply["material"]["default"], "MDF");
    assert_eq!(reply["match"], json!({"visible": false}));
}

#[test]
fn the_edit_hook_reads_body_names_and_the_picked_cabinets_and_never_fails() {
    let mut asked = Vec::new();
    let mut run = |command: &str, args: Value| {
        asked.push(json!([command, args]));
        Ok(
            json!({"bodies": ["C Side L"], "params": [{"name": "C_return"}],
                  "selection": [{"type": "body", "name": "C Side L"}]}),
        )
    };
    let reply = dialog("edit", &json!({}), &mut run).unwrap();
    // the design's selection (the field is empty), our files (none here), so only the other
    // kind hidden
    assert_eq!(
        asked,
        [
            json!(["_selection", {}]),
            json!(["storage.get", {"plugin": "woodwork"}]),
            json!(["document.inspect", {"bodies": "names", "timeline": false}])
        ]
    );
    assert_eq!(reply["doors"], json!({"visible": false}));
    // no default anywhere: in Edit an empty field means unchanged
    assert!(!reply.to_string().contains("default"), "{reply}");
    let mut failing = |_: &str, _: Value| -> Result<Value, String> { Err("busy".into()) };
    assert_eq!(dialog("edit", &json!({}), &mut failing).unwrap(), json!({}));
}

#[test]
fn see_through_boards_change_our_materials_and_new_ones() {
    // On: our design materials that are solid get the opacity with their look; Oak (built in)
    // and one already see-through are left.
    let list = json!({"materials": [
        {"name": "MDF", "user": true, "appearance": {"color": "#b59a73", "opacity": 1.0}},
        {"name": "White melamine", "user": true, "appearance": {"opacity": 0.35}},
        {"name": "Birch plywood", "user": true, "appearance": {"name": "Wood"}},
        {"name": "Oak"}, {"name": "Steel", "user": true}]});
    let stored = json!({"document": {"settings": {"see_through": "on"}}});
    let mut edits = Vec::new();
    let mut run = |command: &str, args: Value| {
        match command {
            "material.list" => return Ok(list.clone()),
            "storage.get" => return Ok(stored.clone()),
            "material.edit" => edits.push(args),
            _ => {}
        }
        Ok(json!({}))
    };
    let reply = execute("settings", &json!({"see_through": "on"}), &mut run).unwrap();
    assert_eq!(reply["see_through"], json!(["MDF", "Birch plywood"]));
    assert_eq!(
        edits[0],
        json!({"material": "MDF", "color": "#b59a73", "opacity": 0.35, "pbr": null})
    );
    assert_eq!(edits[1]["appearance"], "Wood - Cream Birch Uniform Preview");
    // a material made later is made see-through
    let mut made = Value::Null;
    let mut run = |command: &str, args: Value| {
        match command {
            "storage.get" => return Ok(stored.clone()),
            "material.create" => made = args,
            _ => {}
        }
        Ok(json!({}))
    };
    material::assign(&mut run, "Oak", &["A Side L".into()]).unwrap();
    assert_eq!(made["opacity"], 0.35);
    // off: solid again
    assert_eq!(material::opacity(&json!({})), 1.0);
}

#[test]
fn a_cabinet_keeps_its_edit_values_in_a_small_file_not_in_our_document() {
    // Every hook parses our main document with 10M fuel (the Cabinet dialog ran out at 30
    // cabinets when each kept its values there): a cabinet run writes none of it, and its Edit
    // values go to a file of its own, small.
    let mut called = Vec::new();
    let mut run = |command: &str, args: Value| {
        called.push((command.to_string(), args));
        Ok(json!({}))
    };
    execute("cabinet", &json!({"name": "K 2", "drawers": 2}), &mut run).unwrap();
    assert!(!called.iter().any(|(c, _)| c == "storage.patch"));
    let blobs: Vec<&Value> = (called.iter())
        .filter(|(c, _)| c == "_put_blob")
        .map(|(_, a)| a)
        .collect();
    assert_eq!(blobs.len(), 1);
    assert_eq!(blobs[0]["name"], "edit/K_2.json");
    assert!(
        blobs[0]["data"].to_string().len() < 1500,
        "{}",
        blobs[0]["data"]
    );
    // the Edit hook in a 30-cabinet design: our files' names, the picked cabinet's file and its
    // sizes, never the whole design (whose names and parameters take most of the hook's 10M fuel)
    let mut asked = Vec::new();
    let files: Vec<Value> = (0..30)
        .map(|i| json!({"name": format!("edit/K{i}.json")}))
        .collect();
    let mut run = |command: &str, _: Value| {
        asked.push(command.to_string());
        Ok(json!({"blobs": files, "entry": "cabinet", "shown": {"doors": 1}}))
    };
    let pick = json!({"selection": [{"type": "body", "name": "K12 Side L"}]});
    let reply = dialog("edit", &pick, &mut run).unwrap();
    assert_eq!(asked, ["storage.get", "_get_blob", "_params"]);
    assert_eq!(reply["doors"], json!({"default": 1}));
}

/// What `make` wrote to a cabinet's file (`_put_blob`), for a run of `entry` with `params`.
fn kept_file(entry: &str, params: Value) -> (String, Value) {
    let mut put = None;
    let mut run = |command: &str, args: Value| {
        if command == "_put_blob" {
            put = Some((
                args["name"].as_str().unwrap().to_string(),
                args["data"].clone(),
            ));
        }
        Ok(json!({}))
    };
    execute(entry, &params, &mut run).unwrap();
    put.unwrap()
}

/// The Edit hook's reply for boards picked in its field, with these files in the design.
fn edit_hook(files: &[(String, Value)], picks: &[&str]) -> Value {
    let picks: Vec<Value> = (picks.iter())
        .map(|b| json!({"type": "body", "name": b}))
        .collect();
    let mut run = |command: &str, args: Value| match command {
        "storage.get" => {
            let names: Vec<Value> = files.iter().map(|(n, _)| json!({"name": n})).collect();
            Ok(json!({"blobs": names}))
        }
        "_get_blob" => Ok((files.iter())
            .find(|(n, _)| args["name"] == n.as_str())
            .map(|(_, d)| d.clone())
            .unwrap_or_default()),
        // the sizes as they are in the design now
        "_params" => Ok(json!({"C_width": 450.0, "C_drawer_height": 180.0})),
        _ => Ok(json!({"bodies": [], "selection": []})),
    };
    dialog("edit", &json!({"selection": picks}), &mut run).unwrap()
}

#[test]
fn the_edit_hook_starts_at_what_the_last_edit_built() {
    // An Edit builds the cabinet again through `make` with the merged values, which writes its
    // file: the next Edit starts at them, the drawer front height too (#129: it was left empty).
    let file = kept_file(
        "cabinet",
        json!({"name": "C", "drawers": 1, "doors": 1, "drawer_height": 180, "plinth": 100}),
    );
    let reply = edit_hook(&[file], &["C Top"]);
    assert_eq!(reply["drawers"], json!({"default": 1}), "{reply}");
    assert_eq!(reply["doors"], json!({"default": 1}));
    assert_eq!(reply["drawer_height"], json!({"default": 180.0}));
    assert_eq!(reply["width"], json!({"default": 450.0}));
    assert_eq!(reply["return"], json!({"visible": false}));
}

#[test]
fn a_cabinet_without_a_file_never_takes_another_cabinets() {
    // "Cabinet 2" made before has no file; "Cabinet"'s name starts its boards' names, but its
    // file does not list them (#129: Edit showed Drawers 2 for a cabinet with 1).
    let file = kept_file("cabinet", json!({"name": "Cabinet", "drawers": 2}));
    let reply = edit_hook(std::slice::from_ref(&file), &["Cabinet 2 Top"]);
    assert!(reply.get("drawers").is_none(), "{reply}");
    assert_eq!(
        edit_hook(&[file], &["Cabinet Top"])["drawers"],
        json!({"default": 2})
    );
}

#[test]
fn edit_changes_one_cabinet_and_first_writes_the_file_of_one_made_before() {
    let side = |name: &str, x: f64| json!({"name": name, "bbox": {"min": [x, -280, 0], "max": [x + 18.0, 280, 720]}});
    let edit = |picks: Value, file: Value| {
        let mut called: Vec<String> = Vec::new();
        let mut run = |command: &str, _: Value| {
            called.push(command.to_string());
            Ok(match command {
                "document.inspect" => json!({"selection": picks, "params": []}),
                "inspect.measure" => {
                    json!({"bodies": [side("A Side L", 0.0), side("B Side L", 900.0)]})
                }
                "component.metadata" => json!({"metadata": {edit::KEY: {"entry": "cabinet",
                    "params": {"name": "A", "doors": 1, "shelves": 3}}}}),
                "component.list" => json!({"components": [{"id": 1, "name": "A",
                    "occurrences": [{"id": 2, "parent": 0, "transform": [[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]]}]}]}),
                "_get_blob" => file.clone(),
                _ => json!({}),
            })
        };
        // as the dialog sends it: every field, at its default where nothing could be shown
        let dialog = json!({"doors": 2, "shelves": 1, "width": 600, "drawers": 0, "handles": "none",
            "plinth": "yes", "shelf_pins": false, "labels": "no", "fronts": "overlay"});
        let reply = execute("edit", &dialog, &mut run);
        (called, reply)
    };
    let (_, reply) = edit(
        json!([{"name": "A Side L"}, {"name": "B Side L"}]),
        json!({}),
    );
    assert!(reply.unwrap_err().starts_with("Edit changes one cabinet"));
    // made before Edit's file: the dialog could not show its values, so a field at its default
    // is unchanged (3 shelves stay); Doors 2 is built in the same run, and the file written
    let (called, reply) = edit(json!([{"name": "A Side L"}]), Value::Null);
    let reply = reply.unwrap();
    assert_eq!(reply["edited"], true, "{reply}");
    assert_eq!(reply["changed"], json!(["doors"]));
    assert!(called.contains(&"_put_blob".to_string()));
    assert!(called.contains(&"component.delete".to_string()));
}

#[test]
fn cabinet_with_mode_edit_is_edit() {
    // the dialog: a box cabinet's fields as it is, its Shape greyed on Box
    let file = kept_file("cabinet", json!({"name": "C", "doors": 1}));
    let mut run = |command: &str, _: Value| match command {
        "storage.get" => Ok(json!({"blobs": [{"name": file.0}]})),
        "_get_blob" => Ok(file.1.clone()),
        _ => Ok(json!({})),
    };
    let pick = json!({"mode": "edit", "selection": [{"type": "body", "name": "C Top"}]});
    let reply = dialog("cabinet", &pick, &mut run).unwrap();
    assert_eq!(reply["doors"], json!({"default": 1}), "{reply}");
    assert_eq!(reply["shape"], json!({"default": "box", "enabled": false}));
    // the run: Edit's (a board that is no cabinet's is refused as Edit refuses it)
    let mut run = |_: &str, _: Value| Ok(json!({"selection": [{"name": "Loose"}], "bodies": []}));
    let err = execute("cabinet", &json!({"mode": "edit"}), &mut run).unwrap_err();
    assert!(err.contains("not part of a cabinet"), "{err}");
}

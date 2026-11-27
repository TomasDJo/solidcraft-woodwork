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
    // the internal entry and every section's (Doors since #211)
    for entry in [
        "settings",
        "settings-fronts",
        "settings-doors",
        "settings-display",
    ] {
        assert_eq!(
            calls(entry),
            [
                "storage.get",
                "storage.patch",
                "storage.get",
                "material.list"
            ],
            "{entry}"
        );
    }
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
fn a_selected_sketch_line_sets_place_line_and_the_line_in_the_hook() {
    // #212: the design's selection, a sketch line: Place Along a line, on it
    let mut run = |command: &str, _: Value| {
        Ok(match command {
            "document.inspect" => json!({"bodies": ["A Side L"], "selection": [
                {"type": "sketch_curve", "sketch": 2, "id": "l1"}]}),
            _ => json!({}),
        })
    };
    let reply = dialog("cabinet", &json!({"mode": "new"}), &mut run).unwrap();
    assert_eq!(reply["place"], json!({"default": "line"}));
    let pick = json!({"type": "external_sketch_curve", "sketch": 2, "id": "l1"});
    assert_eq!(reply["line"], json!({"default": [pick]}));
    assert_eq!(reply["height_line"], json!({"default": []}));
    // a cabinet's board selected: no answer for Place, so it stays Auto
    let mut run = |command: &str, _: Value| {
        Ok(match command {
            "document.inspect" => {
                json!({"bodies": ["A Side L"], "selection": [{"name": "A Side L"}]})
            }
            _ => json!({}),
        })
    };
    let reply = dialog("cabinet", &json!({"mode": "new"}), &mut run).unwrap();
    assert!(
        reply.get("place").is_none() && reply.get("line").is_none(),
        "{reply}"
    );
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

#[test]
fn mode_auto_goes_by_what_is_picked_in_the_dialog_and_the_run() {
    let names = json!({"bodies": ["C Side L", "C Top", "C Door", "C Drawer 1 Front", "C Shelf 1"]});
    let pick = |b: &str| vec![b.to_string()];
    assert_eq!(
        select::auto(&names, &pick("C Door")),
        select::Auto::Edit("C", "a door")
    );
    assert_eq!(
        select::auto(&names, &pick("C Drawer 1 Front")).hint(),
        "Auto: editing C (a drawer front is picked)"
    );
    assert_eq!(
        select::auto(&names, &pick("C Side L")).hint(),
        "Auto: a new cabinet next to C"
    );
    assert_eq!(select::auto(&names, &[]).hint(), "Auto: a new cabinet");
    // the dialog: a door picked shows Edit's fields, prefilled, the new cabinet's hidden; a
    // side picked shows the new cabinet's, Edit's hidden
    let file = kept_file("cabinet", json!({"name": "C", "doors": 2}));
    let dialog_with = |body: &str| {
        let (file, body) = (file.clone(), body.to_string());
        let mut run = move |command: &str, _: Value| match command {
            "storage.get" => Ok(json!({"blobs": [{"name": file.0}]})),
            "_get_blob" => Ok(file.1.clone()),
            "document.inspect" | "_selection" => {
                Ok(json!({"bodies": ["C Side L", "C Top", "C Door L"],
                "params": [], "selection": [{"type": "body", "name": body}]}))
            }
            _ => Ok(json!({})),
        };
        dialog("cabinet", &json!({"mode": "auto"}), &mut run).unwrap()
    };
    let reply = dialog_with("C Door L");
    assert_eq!(
        reply["mode"],
        json!({"hint": "Auto: editing C (a door is picked)"})
    );
    assert_eq!(reply["doors"], json!({"default": 2}), "{reply}");
    assert_eq!(reply["name"], json!({"visible": false}));
    // Size by stays, so Width and Depth show or hide by it, as in Edit picked
    assert!(reply["sizing"].get("visible").is_none(), "{reply}");
    let reply = dialog_with("C Side L");
    assert_eq!(
        reply["mode"],
        json!({"hint": "Auto: a new cabinet next to C"})
    );
    assert_eq!(reply["fronts"], json!({"visible": false}));
    assert!(reply["name"].get("visible").is_none(), "{reply}");
    // the run decides the same: a door picked edits C (a board of no cabinet is refused as
    // Edit refuses it), a side picked makes a new cabinet
    let run_with = |body: &str| {
        let body = body.to_string();
        let mut called = Vec::new();
        let mut run = |command: &str, _: Value| {
            called.push(command.to_string());
            Ok(match command {
                "document.inspect" => json!({"selection": [{"name": body}], "params": []}),
                "inspect.measure" => {
                    json!({"bodies": [{"name": "C Side L"}, {"name": "C Door L"}]})
                }
                _ => json!({}),
            })
        };
        let reply = execute(
            "cabinet",
            &json!({"mode": "auto", "place": "origin", "fronts": "inset"}),
            &mut run,
        );
        (called, reply)
    };
    let (called, _) = run_with("C Door L");
    assert!(
        called.contains(&"component.metadata".to_string()),
        "{called:?}"
    );
    let (called, reply) = run_with("Loose");
    assert!(
        reply.is_ok() && !called.contains(&"component.metadata".to_string()),
        "{called:?}"
    );
}
#[test]
fn the_cabinet_dialog_says_at_once_what_the_build_would_refuse() {
    // an app with nothing stored and nothing picked
    let mut run = |_: &str, _: Value| Ok(json!({}));
    let reply = dialog("cabinet", &json!({"shape": "box", "doors": 3}), &mut run).unwrap();
    let hint = reply["doors"]["hint"].as_str().unwrap();
    assert!(hint.contains("dividers + 1 = 1 here"), "{hint}");
    assert!(hint.contains("nothing is made or previewed"), "{hint}");
    // no Problem row any more (removed for 0.5.0): the reply names no such field
    assert!(reply.get("problem").is_none(), "{reply}");
    // a corner's doors are Doors there; a fine cabinet gets no hint
    let ok = dialog("cabinet", &json!({"shape": "box", "doors": 2}), &mut run).unwrap();
    assert!(ok["doors"].get("hint").is_none(), "{ok}");
    let low = dialog(
        "cabinet",
        &json!({"shape": "box", "height": 100, "drawers": 4}),
        &mut run,
    );
    let low = low.unwrap();
    assert!(low["height"]["hint"].is_string(), "{low}");
    // Woodwork settings: casters on a plinth too low for them, said on the plinth's height
    let values = json!({"plinth": 40, "plinth_type": "casters"});
    let reply = dialog("settings", &values, &mut run).unwrap();
    let hint = reply["plinth"]["hint"].as_str().unwrap_or_default();
    assert!(
        hint.contains("Casters need a plinth") && hint.contains("new cabinets"),
        "{reply}"
    );
}

#[test]
fn placing_measures_only_the_picked_cabinet_and_lands_where_a_full_measure_put_it() {
    let body = |name: &str, x: f64| {
        json!({"name": name, "material": "Oak", "volume_mm3": 18.0 * 560.0 * 720.0,
        "bbox": {"min": [x, -280, 0], "max": [x + 18.0, 280, 720]}})
    };
    let all = json!([
        body("A Side L", -300.0),
        body("A Side R", 282.0),
        body("A Top", -282.0),
        body("Z Side L", 2000.0),
        body("Z Side R", 2582.0)
    ]);
    // `narrow`: the app as it is (names only, or just the bodies asked for); else every body
    // measured whatever is asked, as the full `inspect.measure` the run made before
    let place = |narrow: bool| {
        let (mut moved, mut asked) = (Value::Null, Vec::new());
        let mut run = |command: &str, args: Value| {
            if command == "occurrence.move" {
                moved = args.clone();
            }
            Ok(match command {
                "document.inspect" => json!({"selection": [{"name": "A Side R"}]}),
                "inspect.measure" if narrow && args["names_only"] == true => {
                    let names = all
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|b| json!({"name": b["name"]}));
                    json!({"bodies": names.collect::<Vec<_>>()})
                }
                "inspect.measure" if narrow => {
                    asked.push(args["bodies"].clone());
                    let wanted = args["bodies"].as_array().unwrap();
                    let bodies = all
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter(|b| wanted.contains(&b["name"]));
                    json!({"bodies": bodies.cloned().collect::<Vec<_>>()})
                }
                "inspect.measure" => json!({"bodies": all}),
                _ => json!({}),
            })
        };
        let reply = execute("cabinet", &json!({"place": "auto"}), &mut run).unwrap();
        (reply["placed"].clone(), moved, asked)
    };
    let (placed, moved, asked) = place(true);
    assert_eq!(asked, [json!(["A Side L", "A Side R", "A Top"])]);
    let (full_placed, full_moved, _) = place(false);
    assert_eq!((&placed, &moved), (&full_placed, &full_moved));
    assert_eq!(placed, "right of A");
    assert_eq!(moved["translate"][0], json!(300.0 + 300.0));
}

#[test]
fn edit_picked_says_a_refused_change_on_its_field() {
    // Cabinet A, 900 wide with two dividers and three doors, kept in its Edit file; the dialog
    // in Edit picked asks for four doors (more than one per compartment)
    let mut run = |command: &str, _: Value| {
        Ok(match command {
            "storage.get" => json!({"blobs": [{"name": "edit/A.json"}]}),
            "_get_blob" => json!({"entry": "cabinet", "units": "mm", "parts": ["Side"],
                "shown": {"doors": 3, "dividers": 2, "shelves": 1, "kind": "standard"}}),
            "_params" => json!({"A_width": 900.0, "A_height": 720.0, "A_depth": 560.0}),
            _ => json!({}),
        })
    };
    let picked = json!([{"type": "body", "name": "A Side L"}]);
    let values = json!({"mode": "edit", "selection": picked, "doors": 4, "dividers": 2});
    let reply = dialog("cabinet", &values, &mut run).unwrap();
    let text = reply["doors"]["hint"].as_str().unwrap_or_default();
    assert!(text.contains("dividers + 1 = 3 here"), "{reply}");
    assert!(reply.get("problem").is_none(), "{reply}");
    // as it is: no hint
    let values = json!({"mode": "edit", "selection": picked, "doors": 3, "dividers": 2});
    let reply = dialog("cabinet", &values, &mut run).unwrap();
    assert!(reply["doors"].get("hint").is_none(), "{reply}");
}

#[test]
fn mode_auto_says_a_refused_change_when_editing_and_when_new() {
    // Cabinet A, 900 wide with two dividers and three doors; Mode Auto with its door picked
    // edits it (`select::auto`), and four doors are more than one per compartment
    let mut run = |command: &str, _: Value| {
        Ok(match command {
            "document.inspect" => json!({"bodies": [{"name": "A Side L"}, {"name": "A Side R"},
                {"name": "A Door 1"}]}),
            "storage.get" => json!({"blobs": [{"name": "edit/A.json"}]}),
            "_get_blob" => json!({"entry": "cabinet", "units": "mm", "parts": ["Side", "Door"],
                "shown": {"doors": 3, "dividers": 2, "shelves": 1, "kind": "standard"}}),
            "_params" => json!({"A_width": 900.0, "A_height": 720.0, "A_depth": 560.0}),
            _ => json!({}),
        })
    };
    let door = json!([{"type": "body", "name": "A Door 1"}]);
    let values = json!({"mode": "auto", "selection": door, "doors": 4, "dividers": 2});
    let reply = dialog("cabinet", &values, &mut run).unwrap();
    let hint = reply["mode"]["hint"].as_str().unwrap_or_default();
    assert!(hint.contains("editing A"), "{reply}");
    let text = reply["doors"]["hint"].as_str().unwrap_or_default();
    assert!(text.contains("dividers + 1 = 3 here"), "{reply}");
    // Auto with nothing picked is a new cabinet: three doors and no dividers are refused there
    let values = json!({"mode": "auto", "shape": "box", "doors": 3, "dividers": 0});
    let reply = dialog("cabinet", &values, &mut run).unwrap();
    assert_eq!(reply["mode"]["hint"], "Auto: a new cabinet", "{reply}");
    let text = reply["doors"]["hint"].as_str().unwrap_or_default();
    assert!(text.contains("dividers + 1 = 1 here"), "{reply}");
}

#[test]
fn a_cabinet_not_along_a_line_remembers_no_line() {
    // A script's stale line fields with Place At the origin: not kept, or Apply settings would
    // move the cabinet onto that line.
    let mut kept = Value::Null;
    let mut run = |command: &str, args: Value| {
        if command == "component.metadata.set" {
            kept = args["values"].clone();
        }
        Ok(json!({}))
    };
    let params = json!({"place": "origin", "line": [{"type": "edge", "body": "Wall", "index": 1}],
        "height_line": [], "start_at": "right", "offset": 5, "faces": "flip", "width_from_line": true});
    execute("cabinet", &params, &mut run).unwrap();
    let remembered = &kept[edit::KEY]["params"];
    for key in LINE_FIELDS {
        assert!(remembered.get(key).is_none(), "{key}: {remembered}");
    }
}

#[test]
fn along_a_line_names_its_dimension_and_sizes_the_width_by_it() {
    // A sketch line 1200 long in an inch design's dialog: the offset is in inches, the app's
    // `d1` is renamed after the sketch, and the width is an expression over it.
    let sketch = json!({
        "name": "Wall line",
        "curves": [{"id": "l1", "type": "line", "start": "l1.start", "end": "l1.end"}],
        "points": [{"id": "l1.start", "world": [0.0, 0.0, 0.0]}, {"id": "l1.end", "world": [1200.0, 0.0, 0.0]}],
        "constraints": [{"type": "length", "l": 0, "param": "d1"}],
    });
    let mut called: Vec<(String, Value)> = Vec::new();
    let mut run = |command: &str, args: Value| {
        called.push((command.to_string(), args));
        Ok(sketch.clone())
    };
    let mut params = json!({"units": "in", "offset": 2, "start_at": "left", "width_from_line": true,
        "line": [{"type": "external_sketch_curve", "sketch": 1, "id": "l1"}]});
    let design = json!({"params": [], "sketches": [{"id": 1}, {"id": 2}]});
    let (_, front, follows) = along_line(&mut params, &design, &mut run).unwrap();
    assert_eq!(front, [0.0, -1.0]);
    assert!(follows.contains("width follows Wall_line_l1"), "{follows}");
    assert_eq!(called[0].0, "sketch.inspect");
    assert_eq!(
        called[1],
        (
            "parameters.rename".into(),
            json!({"name": "d1", "new_name": "Wall_line_l1"})
        )
    );
    assert_eq!(params["_line"]["width"], "Wall_line_l1 - 2 in");
    // (1200 mm - 2 in) in inches, rounded
    assert_eq!(params["width"], json!(45.244094));
    // a dimension already named is used as it is; no pick, or drawer sizing, is refused
    let mut named = sketch.clone();
    named["constraints"][0]["param"] = json!("Run");
    let mut run = |_: &str, _: Value| Ok(named.clone());
    let mut p = json!({"width_from_line": true, "line": [{"type": "external_sketch_curve", "sketch": 1, "id": "l1"}],
        "height_line": [{"type": "external_sketch_curve", "sketch": 2, "id": "l1"}]});
    let (_, _, said) = along_line(&mut p, &design, &mut run).unwrap();
    assert_eq!(p["_line"]["width"], "Run");
    // Height from line: as tall as that line, following its dimension
    assert_eq!(
        (p["height"].clone(), p["_line"]["height"].clone()),
        (json!(1200.0), json!("Run"))
    );
    assert!(said.contains("height follows Run"), "{said}");
    let mut p = json!({"width_from_line": true, "sizing": "drawer",
        "line": [{"type": "external_sketch_curve", "sketch": 1, "id": "l1"}]});
    assert!(along_line(&mut p, &design, &mut run)
        .unwrap_err()
        .contains("drawers' inside"));
    let mut p = json!({"line": []});
    assert!(along_line(&mut p, &design, &mut run)
        .unwrap_err()
        .contains("pick a line"));
    let pick = json!([{"type": "external_sketch_curve", "sketch": 1, "id": "l1"}]);
    let mut p = json!({"width_from_line": true, "offset": 1300, "line": pick});
    let err = along_line(&mut p, &design, &mut run).unwrap_err();
    assert!(err.contains("shorter than the offset"), "{err}");
    // a sketch no longer in the design: refused before any command, which would end the run
    let mut p = json!({"line": [{"type": "external_sketch_curve", "sketch": 7, "id": "l1"}]});
    let err = along_line(&mut p, &design, &mut run).unwrap_err();
    assert!(err.contains("no longer in the design"), "{err}");
    let mut p = json!({"start_at": "end", "line": pick});
    let err = along_line(&mut p, &design, &mut run).unwrap_err();
    assert!(err.contains("start_at must be"), "{err}");
}

#[test]
fn apply_settings_says_why_a_cabinet_stayed_off_its_line() {
    // A cabinet along a line whose sketch is gone: built again where it stood, with the reason
    // in the reply's `lines` (end to end: tests/test_place.py, with the wall suppressed).
    let mut run = |command: &str, _: Value| {
        Ok(match command {
            "document.inspect" => json!({"sketches": [], "params": [
                {"name": "B_width", "value": 600.0}, {"name": "B_height", "value": 720.0},
                {"name": "B_depth", "value": 560.0}, {"name": "B_thickness", "value": 18.0}]}),
            "inspect.measure" => json!({"bodies": [{"name": "B Side L"}]}),
            "component.metadata" => json!({"metadata": {edit::KEY: {"entry": "cabinet",
                "params": {"name": "B", "line": [{"type": "external_sketch_curve", "sketch": 7, "id": "l1"}]}}}}),
            "component.list" => json!({"active": 0, "components": [{"name": "B", "occurrences": [
                {"id": 1, "parent": 0, "transform": [[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]]}]}]}),
            "joint.list" => json!({"joints": []}),
            _ => json!({}),
        })
    };
    let reply = apply(&json!({}), &json!({}), true, true, &mut run).unwrap();
    assert_eq!(reply["applied"], json!(["B"]), "{reply}");
    let why = reply["lines"]["B"].as_str().unwrap_or_default();
    assert!(
        why.contains("no longer in the design") && why.contains("left where it stood"),
        "{reply}"
    );
}

#[test]
fn a_base_refuses_a_cabinet_that_would_not_stand_on_it() {
    // B is held to a cabinet that is not picked (not grounded): it does not move, so the run is
    // refused, naming B, and nothing is left made
    let side = |name: &str, x: f64| json!({"name": name, "bbox": {"min": [x, 0.0, 0.0], "max": [x + 18.0, 560.0, 720.0]}});
    let bodies = json!({"bodies": [side("B Side L", 0.0), side("B Side R", 582.0)]});
    let mut run = |command: &str, _: Value| {
        Ok(match command {
            "inspect.measure" => bodies.clone(),
            "component.list" => json!({"active": 0, "components": [{"name": "B", "occurrences": [
                {"id": 1, "parent": 0, "grounded": false,
                 "transform": [[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]]}]}]}),
            _ => json!({}),
        })
    };
    let params = json!({"name": "Bench", "selection": ["B Side L"]});
    let err = execute("base", &params, &mut run).unwrap_err();
    assert!(err.contains("`B` did not go up onto the base"), "{err}");
}

/// `component.list` with cabinet A (occurrence 7, at x 100, y 200, z 170) and the base Bench (3).
fn on_a_base() -> Value {
    let at = json!([[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0], [100, 200, 170, 1]]);
    json!({"components": [
        {"name": "A", "occurrences": [{"id": 7, "transform": at, "grounded": true}]},
        {"name": "Bench", "occurrences": [{"id": 3, "grounded": true}]},
    ]})
}

#[test]
fn a_cabinet_is_held_to_its_base_and_notes_it() {
    let mut asked = Vec::new();
    let mut run = |command: &str, args: Value| {
        asked.push(json!([command, args]));
        Ok(match command {
            "component.list" => on_a_base(),
            "component.metadata" => json!({"metadata": {edit::KEY: {"entry": "cabinet"}}}),
            _ => json!({}),
        })
    };
    on_base("Bench", "A", &mut run).unwrap();
    let find = |name: &str| asked.iter().find(|a| a[0] == name).map(|a| a[1].clone());
    // no longer grounded: it rides on the base
    assert_eq!(
        find("occurrence.ground").unwrap(),
        json!({"occurrence": "A:1", "grounded": false})
    );
    let joint = find("joint.create").unwrap();
    assert_eq!(
        (&joint["type"], &joint["name"]),
        (&json!("rigid"), &json!("A link"))
    );
    assert_eq!(
        (&joint["a"]["occurrence"], &joint["b"]["occurrence"]),
        (&json!("Bench:1"), &json!("A:1"))
    );
    // both frames at the cabinet's origin, so nothing moves
    assert_eq!(joint["a"]["frame"]["origin"], json!([100.0, 200.0, 170.0]));
    let noted = find("component.metadata.set").unwrap();
    assert_eq!(
        noted["values"][edit::KEY]["link"],
        json!({"to": "Bench", "place": "base"})
    );
}

#[test]
fn only_a_base_still_there_places_a_cabinet_again() {
    let noted = |to: &str, place: &str| json!({"metadata": {edit::KEY: {"link": {"to": to, "place": place}}}});
    let tree = on_a_base();
    assert_eq!(live_base(&noted("Bench", "base"), &tree), Some("Bench"));
    // a base deleted since: the cabinet goes back as it stood alone (its line, if any)
    assert_eq!(live_base(&noted("Gone", "base"), &tree), None);
    // a link to a neighbour is not a base's
    assert_eq!(live_base(&noted("Bench", "right"), &tree), None);
    assert_eq!(live_base(&json!({}), &tree), None);
}

#[test]
fn a_base_made_again_grounds_the_cabinets_it_held_first() {
    let mut asked = Vec::new();
    let mut run = |command: &str, args: Value| {
        asked.push(json!([command, args]));
        Ok(match command {
            "joint.list" => json!({"joints": [{"name": "A link", "a": 3, "b": 7}]}),
            _ => json!({}),
        })
    };
    release("Bench", &on_a_base(), &mut run).unwrap();
    assert_eq!(
        asked[1],
        json!(["occurrence.ground", {"occurrence": "A:1", "grounded": true}])
    );
    assert_eq!(asked.len(), 2);
}

/// An app with cabinet A (occurrence 7) on the base Bench (occurrence 3), held by `A link`, A
/// placed along the sketch line `l1` and noted as held to Bench; `base` false leaves Bench out
/// of the design. Each call is kept with its arguments.
fn on_bench(
    base: bool,
    called: &mut Vec<(String, Value)>,
) -> impl FnMut(&str, Value) -> Result<Value, String> + '_ {
    let side = |name: &str, x: f64| json!({"name": name, "bbox": {"min": [x, -280, 170], "max": [x + 18.0, 280, 890]}});
    let square = json!([[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0], [0, 0, 170, 1]]);
    let mut components = vec![
        json!({"id": 1, "name": "A", "occurrences": [{"id": 7, "parent": 0, "transform": square, "grounded": false}]}),
    ];
    if base {
        components.push(json!({"id": 2, "name": "Bench", "occurrences": [{"id": 3, "parent": 0, "transform": square, "grounded": true}]}));
    }
    let line = json!([{"type": "external_sketch_curve", "sketch": 1, "id": "l1"}]);
    move |command: &str, args: Value| {
        called.push((command.to_string(), args.clone()));
        Ok(match command {
            "document.inspect" => json!({"selection": [{"name": "A Side L"}], "params": [],
                "sketches": [{"id": 1, "name": "Wall line"}],
                "bodies": [{"name": "A Side L"}, {"name": "A Side R"}]}),
            "inspect.measure" => {
                json!({"bodies": [side("A Side L", 0.0), side("A Side R", 582.0)]})
            }
            "sketch.inspect" => json!({
                "curves": [{"id": "l1", "type": "line", "start": "l1.start", "end": "l1.end"}],
                "points": [{"id": "l1.start", "world": [0.0, 500.0, 0.0]}, {"id": "l1.end", "world": [1200.0, 500.0, 0.0]}],
                "constraints": []}),
            "component.metadata" if args["component"] == "Bench" => {
                json!({"metadata": {edit::KEY: {"entry": "base", "params": {"name": "Bench"}, "cabinets": ["A"]}}})
            }
            "component.metadata" => json!({"metadata": {edit::KEY: {"entry": "cabinet",
                "params": {"name": "A", "doors": 1, "line": line, "start_at": "left"},
                "link": {"to": "Bench", "place": "base"}}}}),
            "component.list" => json!({"components": components}),
            "joint.list" => json!({"joints": [{"name": "A link", "a": 3, "b": 7}]}),
            _ => json!({}),
        })
    }
}

#[test]
fn edit_of_a_cabinet_on_a_base_holds_it_there_not_on_its_line() {
    let dialog = json!({"doors": 2, "width": 600, "drawers": 0, "handles": "none", "plinth": "yes",
        "shelf_pins": false, "labels": "no", "fronts": "overlay"});
    let mut called = Vec::new();
    execute("edit", &dialog, &mut on_bench(true, &mut called)).unwrap();
    // back where it stood (the base's height), not moved onto its line, and held to the base
    let moved_onto_line = called
        .iter()
        .any(|(c, a)| c == "occurrence.move" && a.get("translate").is_some());
    assert!(!moved_onto_line, "placed along its line");
    assert!(called
        .iter()
        .any(|(c, a)| c == "occurrence.move" && a.get("to_matrix").is_some()));
    let link = called
        .iter()
        .find(|(c, a)| c == "joint.create" && a["name"] == "A link")
        .map(|c| &c.1);
    assert_eq!(
        link.map(|a| &a["a"]["occurrence"]),
        Some(&json!("Bench:1")),
        "{called:?}"
    );
    // the base gone: the line places it again, and nothing holds it to a base
    let mut called = Vec::new();
    execute("edit", &dialog, &mut on_bench(false, &mut called)).unwrap();
    assert!(called
        .iter()
        .any(|(c, a)| c == "occurrence.move" && a.get("translate").is_some()));
    assert!(!called
        .iter()
        .any(|(c, a)| c == "joint.create" && a["name"] == "A link"));
}

#[test]
fn mobile_base_run_again_grounds_what_it_held_before_deleting_itself() {
    let mut called = Vec::new();
    let _ = execute(
        "base",
        &json!({"name": "Bench"}),
        &mut on_bench(true, &mut called),
    );
    let at = |command: &str, args: Value| called.iter().position(|c| c.0 == command && c.1 == args);
    let grounded = at(
        "occurrence.ground",
        json!({"occurrence": "A:1", "grounded": true}),
    );
    let deleted = at("component.delete", json!({"component": "Bench"}));
    assert!(grounded.is_some() && deleted.is_some(), "{called:?}");
    assert!(grounded < deleted, "grounded after the base was deleted");
}

#[test]
fn apply_settings_puts_a_cabinet_on_a_base_back_on_its_line_and_the_base_under_it() {
    // Edit: the base holds it; Apply settings with the line read: the line, then a new base
    assert_eq!(
        on_base_or_line(Some("Bench"), false, true),
        (Some("Bench"), None)
    );
    assert_eq!(
        on_base_or_line(Some("Bench"), true, true),
        (None, Some("Bench"))
    );
    // a line that cannot be read: the base holds it, also in Apply settings
    assert_eq!(
        on_base_or_line(Some("Bench"), true, false),
        (Some("Bench"), None)
    );
    assert_eq!(on_base_or_line(None, true, true), (None, None));
}

#[test]
fn apply_settings_puts_a_cabinet_on_a_base_back_on_its_line_then_makes_the_base_again() {
    let mut called = Vec::new();
    let _ = execute(
        "rebuild",
        &json!({"selection": []}),
        &mut on_bench(true, &mut called),
    );
    let onto_line =
        (called.iter()).position(|(c, a)| c == "occurrence.move" && a.get("translate").is_some());
    let base_again =
        (called.iter()).position(|(c, a)| c == "component.delete" && a["component"] == "Bench");
    assert!(onto_line.is_some() && base_again.is_some(), "{called:?}");
    assert!(onto_line < base_again);
}

#[test]
fn apply_settings_says_why_a_line_was_not_read_for_a_cabinet_on_a_base() {
    // the line's sketch deleted: A stays on its base, and Apply settings' `lines` says why
    let mut called = Vec::new();
    let mut inner = on_bench(true, &mut called);
    let mut run = |command: &str, args: Value| match command {
        "document.inspect" => Ok(json!({"selection": [], "params": [], "sketches": [],
            "bodies": [{"name": "A Side L"}, {"name": "A Side R"}]})),
        _ => inner(command, args),
    };
    let reply = execute("rebuild", &json!({"selection": []}), &mut run).unwrap();
    let said = reply["lines"]["A"].as_str().unwrap_or_default();
    assert!(
        said.contains("no longer in the design") && said.contains("left where it stood"),
        "{reply}"
    );
    drop(inner);
    // held to the base again, not placed along the line, and the base not made again
    assert!(called
        .iter()
        .any(|(c, a)| c == "joint.create" && a["a"]["occurrence"] == "Bench:1"));
    assert!(!called
        .iter()
        .any(|(c, a)| c == "occurrence.move" && a.get("translate").is_some()));
    assert!(!called
        .iter()
        .any(|(c, a)| c == "component.delete" && a["component"] == "Bench"));
}

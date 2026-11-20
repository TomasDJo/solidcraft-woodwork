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
    let run_with = |drawings: Value| {
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
                    "sheets": [{"id": "old", "name": "Base"}]}),
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
    let (called, reply) = run_with(json!({"drawings": [{"id": "d1", "name": "Woodwork"}]}));
    assert_eq!(
        reply.unwrap(),
        json!({"drawing": "Woodwork", "sheets": ["Base"]})
    );
    let find = |command: &str| called.iter().find(|c| c.0 == command).unwrap().1.clone();
    assert_eq!(find("drawing.open"), json!({"drawing": "d1"}));
    // in mm, whatever the design's units or what the drawing was set to
    assert_eq!(find("drawing.set"), json!({"units": "mm"}));
    assert_eq!(
        find("drawing.sheet.add")["documents"],
        json!({"source": "src", "component": 3})
    );
    assert_eq!(find("drawing.view.base")["sheet"], "new");
    assert_eq!(find("drawing.view.projected")["parent"], "v1");
    let right = called
        .iter()
        .rfind(|c| c.0 == "drawing.dimension.add")
        .unwrap();
    assert_eq!(right.1["view"], "v2");
    // the old sheet goes once the new one is drawn, and the drawing is closed
    let names: Vec<&str> = called.iter().map(|c| c.0.as_str()).collect();
    assert_eq!(
        names[names.len() - 2..],
        ["drawing.sheet.delete", "drawing.close"]
    );
    assert_eq!(
        called[names.len() - 2].1,
        json!({"sheet": "old", "contents": "delete"})
    );
    // no drawing yet: one is made, and its empty first sheet deleted
    let (called, _) = run_with(json!({"drawings": []}));
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

use super::*;

fn design() -> Value {
    json!({"params": [
        {"name": "A_width", "value": 800.0}, {"name": "A_height", "value": 720.0},
        {"name": "A_2_width", "value": 1.0}, // another cabinet's
    ]})
}

#[test]
fn a_cabinet_remembers_how_it_was_made() {
    let params = json!({"width": 600, "doors": 2, "place": "right", "match": "selected",
        "_match": {"prefix": "B"}, "name": "A"});
    let kept = remember("cabinet", &params, "A 2");
    let ours = &kept[KEY];
    assert_eq!(ours["entry"], "cabinet");
    // under the name it got, and without where it was placed or what it was matched to
    assert_eq!(
        ours["params"],
        json!({"width": 600, "doors": 2, "name": "A 2"})
    );
}

#[test]
fn an_edit_keeps_what_is_not_changed() {
    let made = remember(
        "cabinet",
        &json!({"width": 600, "shelves": 1, "doors": 2}),
        "A",
    );
    let changes = json!({"shelves": 3, "doors": null, "handles": "keep", "shelf_pins": "yes",
        "labels": "no", "door_hinge": "right"});
    let (entry, params) = merge(&made[KEY], &changes, &design()).unwrap();
    assert_eq!(entry, "cabinet");
    assert_eq!(params["shelves"], 3);
    assert_eq!(params["doors"], 2); // empty in the dialog: as it was
    assert_eq!(params.get("handles"), None); // keep: as it was, the default
    assert_eq!(params["shelf_pins"], true);
    assert_eq!(params["labels"], false);
    assert_eq!(params["door_hinge"], "right");
    // the width is the design's, where it was changed since, and its parameter is kept
    assert_eq!(params["width"], 800.0);
    assert_eq!(
        params["_existing"],
        json!({"width": 800.0, "height": 720.0})
    );
}

#[test]
fn the_drawer_front_height_and_side_are_kept_or_changed() {
    // made with 140, changed in the design since: the design's value is built again
    let made = remember("cabinet", &json!({"drawers": 3, "drawer_height": 140}), "A");
    let design = json!({"params": [{"name": "A_drawer_height", "value": 160.0}]});
    let (_, params) = merge(&made[KEY], &json!({"doors": 1}), &design).unwrap();
    assert_eq!(
        (&params["drawer_height"], &params["doors"]),
        (&json!(160.0), &json!(1))
    );
    let changes = json!({"drawer_height": 120, "drawers_at": "bottom"});
    let (_, params) = merge(&made[KEY], &changes, &design).unwrap();
    assert_eq!(params["drawer_height"], 120);
    assert_eq!(params["drawers_at"], "bottom");
    assert_eq!(params["_existing"]["drawer_height"], 160.0);
    let (_, params) = merge(&made[KEY], &json!({"drawers_at": "keep"}), &design).unwrap();
    assert_eq!(params.get("drawers_at"), None);
    let corner = remember("corner", &json!({}), "A");
    assert!(merge(&corner[KEY], &json!({"drawer_height": 120}), &design).is_err());
    assert!(merge(&corner[KEY], &json!({"drawers_at": "top"}), &design).is_err());
    // beside doors the empty height was kept as 150: taking the doors away makes them alike
    let mixed = remember("cabinet", &json!({"drawers": 3, "doors": 1}), "A");
    let design = json!({"params": [{"name": "A_drawer_height", "value": 150.0}]});
    let (_, params) = merge(&mixed[KEY], &json!({"doors": 0}), &design).unwrap();
    assert_eq!(params["drawer_height"], 0);
    let changes = json!({"doors": 0, "drawer_height": 140});
    let (_, params) = merge(&mixed[KEY], &changes, &design).unwrap();
    assert_eq!(params["drawer_height"], 140);
    let (_, params) = merge(&mixed[KEY], &json!({"shelves": 2}), &design).unwrap();
    assert_eq!(params["drawer_height"], 150.0);
}

#[test]
fn sizes_come_back_in_the_unit_they_were_given_in() {
    let made = remember("cabinet", &json!({"units": "in", "width": 24}), "A");
    let design = json!({"params": [{"name": "A_width", "value": 609.6}]});
    let (_, params) = merge(&made[KEY], &json!({}), &design).unwrap();
    assert_eq!(params["width"], 24.0);
    assert_eq!(params["_existing"]["width"], 609.6);
    // the reveal stays in mm (the design gives every value in mm)
    let design = json!({"params": [{"name": "A_reveal", "value": 2.0}]});
    let (_, params) = merge(&made[KEY], &json!({}), &design).unwrap();
    assert_eq!(params["reveal"], 2.0);
}

#[test]
fn a_field_of_the_other_kind_of_cabinet_is_refused() {
    let corner = remember("corner", &json!({"doors": "none"}), "A");
    let (_, params) = merge(
        &corner[KEY],
        &json!({"corner_doors": "hinged left"}),
        &design(),
    )
    .unwrap();
    assert_eq!(params["doors"], "hinged left");
    assert!(merge(&corner[KEY], &json!({"dividers": 1}), &design()).is_err());
    let cabinet = remember("cabinet", &json!({}), "A");
    assert!(merge(&cabinet[KEY], &json!({"shape": "diagonal"}), &design()).is_err());
    assert!(merge(&cabinet[KEY], &json!({"corner_doors": "none"}), &design()).is_err());
    // design-wide: Woodwork settings and Apply settings change it
    assert!(merge(&cabinet[KEY], &json!({"joints": "dowels"}), &design()).is_err());
    assert!(merge(&cabinet[KEY], &json!({"reveal": 3}), &design()).is_err());
    let (_, params) = merge(&cabinet[KEY], &json!({"width": 500}), &design()).unwrap();
    assert_eq!(params["width"], 500);
    // made before cabinets remembered, or not by this plugin
    assert!(merge(&Value::Null, &json!({}), &design()).is_err());
}

#[test]
fn where_it_stands() {
    // turned a quarter and moved: the list has the translation in the last row
    let transform = json!([
        [0.0, 1.0, 0.0, 0.0],
        [-1.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [100.0, 200.0, 300.0, 1.0]
    ]);
    let tree = json!({"components": [
        {"name": "Room", "occurrences": [{"parent": 0}]},
        {"name": "A", "occurrences": [{"parent": 7, "grounded": false, "transform": transform}]},
    ]});
    let (parent, matrix, grounded) = placement(&tree, "A").unwrap();
    assert_eq!(parent, 7);
    assert!(!grounded);
    assert_eq!(
        matrix,
        json!([
            [0.0, -1.0, 0.0, 100.0],
            [1.0, 0.0, 0.0, 200.0],
            [0.0, 0.0, 1.0, 300.0]
        ])
    );
    assert!(placement(&tree, "B").is_err());
}

#[test]
fn boards_keep_their_own_material() {
    let measure = json!({"bodies": [
        {"name": "A Door", "material": "Oak"}, {"name": "A Top", "material": "MDF"},
        {"name": "A Back", "material": "Default"}, {"name": "B Door", "material": "Oak"},
    ]});
    let bodies = ["A Door", "A Top", "A Back"].map(String::from);
    let found = materials(&measure, &bodies);
    assert_eq!(found.len(), 2);
    assert_eq!(found["Oak"], ["A Door"]);
    assert_eq!(found["MDF"], ["A Top"]);
}

#[test]
fn a_cabinet_knows_its_links() {
    // B stands against A and C against B: B's own link holds it to A, C's hangs on it
    let tree = json!({"components": [
        {"name": "A", "occurrences": [{"id": 2}]}, {"name": "B", "occurrences": [{"id": 12}]},
        {"name": "C", "occurrences": [{"id": 22}]}, {"name": "B Door", "occurrences": [{"id": 14}]},
    ]});
    let joints = json!({"joints": [
        {"name": "B link", "a": 2, "b": 12}, {"name": "C link", "a": 12, "b": 22},
        {"name": "B Door hinge", "a": 12, "b": 14},
    ]});
    assert_eq!(
        links(&tree, &joints, "B"),
        [("B link".to_string(), true), ("C link".to_string(), false)]
    );
    assert_eq!(hanging(&tree, &joints, "B"), ["C"]);
    assert_eq!(hanging(&tree, &joints, "A"), ["B"]);
    assert!(links(&tree, &joints, "C").iter().all(|link| link.1));
    assert!(hanging(&tree, &joints, "Gone").is_empty());
}

#[test]
fn fronts_and_the_reveal_survive_an_edit() {
    let made = remember(
        "cabinet",
        &json!({"doors": 2, "fronts": "inset", "reveal": 3}),
        "A",
    );
    // the reveal changed in the design since: the design's value wins, the fronts are kept
    let design = json!({"params": [{"name": "A_reveal", "value": 2.5}]});
    let keep = json!({"fronts": "keep", "shelves": 2});
    let (_, params) = merge(&made[KEY], &keep, &design).unwrap();
    assert_eq!(
        (&params["fronts"], &params["reveal"]),
        (&json!("inset"), &json!(2.5))
    );
    assert_eq!(params["_existing"]["reveal"], 2.5);
    let change = json!({"fronts": "overlay"});
    let (_, params) = merge(&made[KEY], &change, &design).unwrap();
    assert_eq!(
        (&params["fronts"], &params["reveal"]),
        (&json!("overlay"), &json!(2.5))
    );
    // a corner takes its fronts too
    let corner = remember("corner", &json!({}), "A");
    let (_, params) = merge(&corner[KEY], &json!({"fronts": "inset"}), &design).unwrap();
    assert_eq!(params["fronts"], "inset");
}

#[test]
fn the_kind_survives_an_edit() {
    let made = remember("cabinet", &json!({"kind": "sink", "doors": 2}), "A");
    let (_, params) = merge(&made[KEY], &json!({"kind": "keep"}), &design()).unwrap();
    assert_eq!(params["kind"], "sink");
    let (_, params) = merge(&made[KEY], &json!({"kind": "standard"}), &design()).unwrap();
    assert_eq!(params["kind"], "standard");
    let corner = remember("corner", &json!({}), "A");
    assert!(merge(&corner[KEY], &json!({"kind": "sink"}), &design()).is_err());
}

#[test]
fn the_plinth_type_survives_an_edit() {
    for entry in ["cabinet", "corner"] {
        let made = remember(entry, &json!({"plinth": 100, "plinth_type": "legs"}), "A");
        let (_, params) = merge(&made[KEY], &json!({"plinth_type": "keep"}), &design()).unwrap();
        assert_eq!(params["plinth_type"], "legs");
        let (_, params) = merge(&made[KEY], &json!({"plinth_type": "notch"}), &design()).unwrap();
        assert_eq!(params["plinth_type"], "notch");
    }
    let made = remember(
        "cabinet",
        &json!({"plinth": 100, "plinth_type": "casters"}),
        "A",
    );
    let (_, params) = merge(&made[KEY], &json!({"plinth_type": "keep"}), &design()).unwrap();
    assert_eq!(params["plinth_type"], "casters");
}

#[test]
fn the_plinth_set_back_survives_an_edit() {
    let made = remember("cabinet", &json!({"units": "in", "plinth_setback": 3}), "A");
    let (_, params) = merge(&made[KEY], &json!({}), &design()).unwrap();
    assert_eq!(params["plinth_setback"], 3);
    // the parameter, where the user may have changed it, in the cabinet's unit
    let design = json!({"params": [{"name": "A_plinth_setback", "value": 101.6}]});
    let (_, params) = merge(&made[KEY], &json!({}), &design).unwrap();
    assert_eq!(params["plinth_setback"], 4.0); // Woodwork settings' field, not Edit's
    assert_eq!(params["_existing"]["plinth_setback"], 101.6);
}

#[test]
fn the_wall_mount_is_kept_or_changed() {
    let made = remember(
        "cabinet",
        &json!({"units": "in", "wall_mount": "cleat"}),
        "A",
    );
    let design = json!({"params": [{"name": "A_cleat_height", "value": 88.9},
                                   {"name": "A_cleat_thickness", "value": 19.05},
                                   {"name": "A_wall_rail", "value": 0.0}]});
    let (_, params) = merge(&made[KEY], &json!({"wall_mount": "keep"}), &design).unwrap();
    assert_eq!(params["wall_mount"], "cleat");
    // the cleat's sizes come back from the design, in the cabinet's unit
    assert_eq!(params["cleat_height"], 3.5);
    assert_eq!(params["cleat_thickness"], 0.75);
    // the steel rail's parameter is known, so a rebuild changes it rather than adds it
    assert_eq!(params["_existing"]["wall_rail"], 0.0);
    let (_, params) = merge(&made[KEY], &json!({"wall_mount": "steel"}), &design).unwrap();
    assert_eq!(params["wall_mount"], "steel");
    // a corner cabinet too, with its return's rail known
    let corner = remember("corner", &json!({}), "A");
    let design = json!({"params": [{"name": "A_wall_rail_return", "value": 0.0}]});
    let (entry, params) = merge(&corner[KEY], &json!({"wall_mount": "rail"}), &design).unwrap();
    assert_eq!(
        (entry.as_str(), &params["wall_mount"]),
        ("corner", &json!("rail"))
    );
    assert_eq!(params["_existing"]["wall_rail_return"], 0.0);
}

#[test]
fn the_hook_hides_what_the_picked_cabinet_does_not_take() {
    // `document.inspect` with body names: a corner C (it has C_return) and a box Base
    let design = json!({
        "bodies": ["C Side L", "C Back return", "Base Side L", "Base Door 1", "F Leg front left", "F Apron L"],
        "params": [{"name": "C_return", "value": 900.0}, {"name": "Base_width", "value": 600.0}],
        "selection": [{"type": "body", "name": "Base Door 1"}],
    });
    let face = |body: &str| json!({"selection": [{"type": "face", "body": body, "index": 2}]});
    let hidden_keys = |reply: Value| {
        let reply = reply.as_object().cloned().unwrap_or_default();
        assert!(reply.values().all(|f| f == &json!({"visible": false})));
        reply.keys().cloned().collect::<Vec<_>>()
    };
    let sorted = |keys: &[&[&str]]| {
        let mut all: Vec<String> = keys.concat().into_iter().map(String::from).collect();
        all.sort();
        all
    };
    let box_only = sorted(&[&BOX_ONLY]);
    assert_eq!(
        hidden_keys(hidden(&face("C Back return"), &design)),
        box_only
    );
    // a corner cabinet is getting the wall mount: shown
    assert!(!box_only.contains(&"wall_mount".to_string()));
    let box_cabinet = sorted(&[&CORNER_ONLY]);
    assert_eq!(
        hidden_keys(hidden(&face("Base Side L"), &design)),
        box_cabinet
    );
    // an empty field falls back to the design's selection, as the run does
    assert_eq!(
        hidden_keys(hidden(&json!({"selection": []}), &design)),
        box_cabinet
    );
    // not a cabinet, or nothing selected: everything shown
    assert_eq!(hidden(&face("Sketch body"), &design), json!({}));
    let nothing = json!({"bodies": ["C Side L"], "params": [{"name": "C_return"}]});
    assert_eq!(hidden(&json!({}), &nothing), json!({}));
}

#[test]
fn every_edit_field_a_kind_takes_is_one_merge_takes() {
    let design = json!({});
    let corner = remember("corner", &json!({}), "C");
    let cabinet = remember("cabinet", &json!({}), "B");
    for key in BOX_ONLY {
        let change = json!({ key: 1 });
        assert!(merge(&corner[KEY], &change, &design).is_err(), "{key}");
        assert!(merge(&cabinet[KEY], &change, &design).is_ok(), "{key}");
    }
    for key in CORNER_ONLY {
        let change = json!({ key: "diagonal" });
        assert!(merge(&cabinet[KEY], &change, &design).is_err(), "{key}");
        assert!(merge(&corner[KEY], &change, &design).is_ok(), "{key}");
    }
    // corner_doors are the corner's doors
    let (_, params) = merge(&corner[KEY], &json!({"corner_doors": "none"}), &design).unwrap();
    assert_eq!(params["doors"], "none");
}

#[test]
fn joinery_sizes_come_from_parameters_and_counts_only_tell_kit() {
    let remembered = json!({"entry": "cabinet", "params": {"name": "K", "joints": "dowels"}});
    let design = json!({"params": [
        {"name": "K_joint_spacing", "value": 120.0}, {"name": "K_dowels", "value": 20.0}
    ]});
    let (_, params) = merge(&remembered, &json!({"shelves": 2}), &design).unwrap();
    assert_eq!(params["joints"], "dowels");
    assert_eq!(params["joint_spacing"], 120.0);
    assert!(params.get("dowels").is_none());
    assert_eq!(params["_existing"]["dowels"], 20.0);
}

#[test]
fn holes_from_the_old_joinery_function_become_the_same_joint_at_build() {
    let remembered = json!({"entry": "cabinet", "params": {"name": "K"}, "joinery": "dowel"});
    let (_, params) = merge(&remembered, &json!({"shelves": 2}), &json!({})).unwrap();
    assert_eq!(params["joints"], "dowels");
    let remembered = json!({"entry": "cabinet", "params": {"name": "K", "joints": "none"},
        "joinery": "confirmat"});
    let (_, params) = merge(&remembered, &json!({}), &json!({})).unwrap();
    assert_eq!(params["joints"], "confirmat");
}

#[test]
fn the_dialog_shows_the_cabinet_as_it_is_and_unchanged_builds_nothing() {
    let made = remember(
        "cabinet",
        &json!({"doors": 2, "drawers": 1, "shelf_pins": true,
        "plinth": 100, "plinth_type": "legs", "fronts": "inset", "drawer_height": 140}),
        "A",
    );
    let design = json!({"params": [{"name": "A_drawer_height", "value": 160.0}]});
    let (entry, base) = merge(&made[KEY], &json!({}), &design).unwrap();
    let shown = shown(&entry, &base);
    assert_eq!(shown["doors"], 2);
    assert_eq!(shown["drawer_height"], 160.0); // from the design, not what it remembered
    assert_eq!(
        (&shown["shelf_pins"], &shown["plinth"]),
        (&json!("yes"), &json!("yes"))
    );
    assert_eq!(shown["fronts"], "inset");
    assert_eq!(shown["labels"], "no");
    // a corner's doors are its corner_doors
    let corner = remember("corner", &json!({"doors": "hinged left"}), "C");
    let (entry, params) = merge(&corner[KEY], &json!({}), &json!({"params": []})).unwrap();
    assert_eq!(edit_shown_doors(&entry, &params), json!("hinged left"));
    // the dialog's values sent back as they are: nothing to build; one change: something
    let mut asis = Map::new();
    for (k, v) in &shown {
        asis.insert(k.clone(), v.clone());
    }
    asis.insert("doors".into(), json!(2.0)); // a number written another way is the same
    asis.insert("plinth".into(), json!(100)); // `settings::edit_plinth` turns Yes into the height
    let (_, params) = merge(&made[KEY], &Value::Object(asis.clone()), &design).unwrap();
    assert!(changed(&params, &base).is_empty());
    asis.insert("drawers".into(), json!(2));
    let (_, params) = merge(&made[KEY], &Value::Object(asis), &design).unwrap();
    assert_eq!(changed(&params, &base), ["drawers"]);
}

#[test]
fn one_cabinet_shows_its_values_and_hides_the_other_kinds_fields() {
    let kept = shown(
        "cabinet",
        &json!({"doors": 1, "drawers": 1, "fronts": "overlay", "joints": "dowels"}),
    );
    let sizes = json!({"width": 450.0, "drawer_height": 180.0});
    let reply = fields("cabinet", &kept, sizes.as_object().unwrap());
    assert_eq!(reply["drawers"], json!({"default": 1}));
    assert_eq!(reply["width"], json!({"default": 450.0}));
    assert_eq!(reply["drawer_height"], json!({"default": 180.0}));
    assert_eq!(reply["corner_doors"], json!({"visible": false}));
    assert_eq!(reply["return"], json!({"visible": false}));
    assert!(reply.get("joints").is_none()); // design-wide: not Edit's
    let corner = shown("corner", &json!({"doors": "hinged left"}));
    let reply = fields("corner", &corner, &Map::new());
    assert_eq!(reply["corner_doors"], json!({"default": "hinged left"}));
    assert_eq!(reply["doors"], json!({"visible": false}));
    // a run leaves out the other kind's fields, which the dialog sends hidden
    let changes = taken(
        "corner",
        &json!({"doors": 0, "corner_doors": "none", "selection": []}),
    );
    assert_eq!(changes, json!({"corner_doors": "none", "selection": []}));
}

fn edit_shown_doors(entry: &str, params: &Value) -> Value {
    shown(entry, params)["corner_doors"].clone()
}

#[test]
fn a_value_left_out_is_the_one_the_dialog_shows() {
    // two doors: Cabinet has no hinge side for them, so none is remembered; Edit shows Left
    let base = json!({"doors": 2});
    assert!(changed(&json!({"doors": 2, "door_hinge": "left"}), &base).is_empty());
    assert_eq!(
        changed(&json!({"doors": 2, "door_hinge": "right"}), &base),
        ["door_hinge"]
    );
}

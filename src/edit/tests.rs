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
    assert!(merge(&cabinet[KEY], &json!({"width": 5}), &design()).is_err());
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
    let keep = json!({"fronts": "keep", "reveal": null, "shelves": 2});
    let (_, params) = merge(&made[KEY], &keep, &design).unwrap();
    assert_eq!(
        (&params["fronts"], &params["reveal"]),
        (&json!("inset"), &json!(2.5))
    );
    assert_eq!(params["_existing"]["reveal"], 2.5);
    let change = json!({"fronts": "overlay", "reveal": 1.5});
    let (_, params) = merge(&made[KEY], &change, &design).unwrap();
    assert_eq!(
        (&params["fronts"], &params["reveal"]),
        (&json!("overlay"), &json!(1.5))
    );
    // a corner takes a reveal and its fronts too
    let corner = remember("corner", &json!({}), "A");
    assert!(merge(&corner[KEY], &json!({"reveal": 3}), &design).is_ok());
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
    let (_, params) = merge(&made[KEY], &json!({"plinth_setback": 4}), &design()).unwrap();
    assert_eq!(params["plinth_setback"], 4);
    // the parameter, where the user may have changed it, in the cabinet's unit
    let design = json!({"params": [{"name": "A_plinth_setback", "value": 101.6}]});
    let (_, params) = merge(&made[KEY], &json!({}), &design).unwrap();
    assert_eq!(params["plinth_setback"], 4.0);
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

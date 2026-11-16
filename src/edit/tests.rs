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
    let changes = json!({"shelves": 3, "doors": null, "handles": "keep", "shelf_pins": "yes"});
    let (entry, params) = merge(&made[KEY], &changes, &design()).unwrap();
    assert_eq!(entry, "cabinet");
    assert_eq!(params["shelves"], 3);
    assert_eq!(params["doors"], 2); // empty in the dialog: as it was
    assert_eq!(params.get("handles"), None); // keep: as it was, the default
    assert_eq!(params["shelf_pins"], true);
    // the width is the design's, where it was changed since, and its parameter is kept
    assert_eq!(params["width"], 800.0);
    assert_eq!(
        params["_existing"],
        json!({"width": 800.0, "height": 720.0})
    );
}

#[test]
fn sizes_come_back_in_the_unit_they_were_given_in() {
    let made = remember("cabinet", &json!({"units": "in", "width": 24}), "A");
    let design = json!({"params": [{"name": "A_width", "value": 609.6}]});
    let (_, params) = merge(&made[KEY], &json!({}), &design).unwrap();
    assert_eq!(params["width"], 24.0);
    assert_eq!(params["_existing"]["width"], 609.6);
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

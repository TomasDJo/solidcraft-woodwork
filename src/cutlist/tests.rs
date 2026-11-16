use super::*;

fn measure() -> Value {
    let body = |name: &str, max: [f64; 3]| json!({"name": name, "material": "Default", "bbox": {"min": [0, 0, 0], "max": max}});
    json!({"bodies": [
        body("Side L", [19.05, 558.8, 762.0]),
        body("Side R", [19.05, 558.8, 762.0]),
        body("Back", [582.0, 6.0, 702.0]),
    ]})
}

#[test]
fn groups_equal_boards() {
    let list = cutlist(&measure(), &BTreeMap::new(), false).unwrap();
    assert_eq!(list["boards"], 3);
    // 6 mm: 0.41 m2, 18 mm: 0.85 m2, one sheet each
    assert_eq!(list["sheets"][0]["thickness"], 6.0);
    assert_eq!(list["sheets"][1]["sheets_2440x1220"], 1.0);
    assert_eq!(list["parts"][0]["thickness"], 6.0); // thinnest first
    assert_eq!(list["parts"][1]["qty"], 2);
    assert_eq!(list["parts"][1]["length"], 762.0);
    let csv = list["csv"].as_str().unwrap();
    assert!(
        csv.contains("2,762,558.8,19.1,\"Default\",\"Side L, Side R\""),
        "{csv}"
    );
}

#[test]
fn an_open_door_is_measured_in_its_own_component() {
    let body = |name: &str, max: [f64; 3], volume: f64| json!({"name": name, "volume_mm3": volume, "bbox": {"min": [0, 0, 0], "max": max}});
    let board = 397.0 * 18.0 * 716.0;
    let measured = json!({"bodies": [
        body("Closed", [397.0, 18.0, 716.0], board - 25000.0), // two hinge cups
        body("Ajar", [397.4, 31.9, 716.0], board),             // 2 degrees open
        body("Open", [296.0, 355.0, 716.0], board),
    ]});
    assert_eq!(turned(&measured), ["Ajar", "Open"]);

    let mesh = json!({"positions": [[0, 0, 0], [397, 18, 716], [397, 0, 0]]});
    let local = BTreeMap::from([("Side L".to_string(), extents(&mesh))]);
    let list = cutlist(&measure(), &local, false).unwrap();
    assert!(list["csv"]
        .as_str()
        .unwrap()
        .contains("1,716,397,18,\"Default\",\"Side L\""));
}

#[test]
fn hardware_is_counted_from_names() {
    let named =
        |names: &[&str]| -> Vec<Value> { names.iter().map(|n| json!({"name": n})).collect() };
    let design = json!({"timeline": named(&[
        "A Door hinge cup 1", "A Door hinge cup 2", "A Door handle 1", "A Door handle 2",
        "A Side L pins", "A Side R pins", "A 2 Drawer 1 Front handle 1",
        "A joint Side L Bottom pilot", "A joint Side L Bottom row",
    ])});
    let mut bodies = named(&[
        "A Shelf 1",
        "A Shelf 2",
        "A 2 Shelf 1",
        "A 2 Drawer 1 Front",
    ]);
    bodies.push(json!({"name": "A Bottom", "face_types": {"cylinder": 6}}));
    bodies.push(json!({"name": "A Top", "face_types": {"cylinder": 6}}));
    bodies.push(json!({"name": "A 2 Top", "face_types": {"cylinder": 3}}));
    let list = hardware(&json!({"bodies": bodies}), &design);
    let expected = json!([
        {"item": "Hinge, 35 mm cup", "qty": 2}, {"item": "Drawer slide, pair", "qty": 1},
        {"item": "Shelf pin, 5 mm", "qty": 8}, {"item": "Confirmat screw, 7 x 50 mm", "qty": 12},
        {"item": "Knob", "qty": 1}, {"item": "Pull, 128 mm", "qty": 1},
    ]);
    assert_eq!(list, expected);
}

#[test]
fn inches_as_fractions() {
    assert_eq!(inches(19.05), "3/4");
    assert_eq!(inches(609.6), "24");
    assert_eq!(inches(590.55), "23 1/4");
    let list = cutlist(&measure(), &BTreeMap::new(), true).unwrap();
    let csv = list["csv"].as_str().unwrap();
    assert!(
        csv.contains("2,30,22,3/4,\"Default\",\"Side L, Side R\""),
        "{csv}"
    );
    assert_eq!(list["parts"][1]["length"], "30");
}

#[test]
fn the_file_to_save_as() {
    let name = |entry: &str, file: &str| file_name(entry, &json!({"file": file}));
    assert_eq!(name("cutlist", ""), Ok(None));
    assert_eq!(name("cutlist", "  "), Ok(None));
    assert_eq!(file_name("cutlist", &json!({})), Ok(None));
    assert_eq!(name("cabinet", "a"), Ok(None)); // only the cut list saves
    assert_eq!(name("cutlist", "kitchen"), Ok(Some("kitchen.csv".into())));
    assert_eq!(
        name("cutlist", " My kitchen 2.CSV "),
        Ok(Some("My kitchen 2.CSV".into()))
    );
    // a name only: the folder is the one the user approved
    for bad in ["../a", "a/b", r"a\b", "C:a", ".hidden"] {
        assert!(name("cutlist", bad).is_err(), "{bad}");
    }
}

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
    let list = cutlist(&measure(), &BTreeMap::new(), false, &json!({})).unwrap();
    assert_eq!(list["boards"], 3);
    // 6 mm: 0.41 m2, 18 mm: 0.85 m2, one sheet each
    assert_eq!(list["sheets"][0]["thickness"], 6.0);
    assert_eq!(list["sheets"][1]["sheets"], 1);
    assert_eq!(list["parts"][0]["thickness"], 6.0); // thinnest first
    assert_eq!(list["parts"][1]["qty"], 2);
    assert_eq!(list["parts"][1]["length"], 762.0);
    let csv = list["csv"].as_str().unwrap();
    assert!(
        csv.contains("2,762,558.8,19.1,\"Default\",,,\"Side L, Side R\""),
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
    let list = cutlist(&measure(), &local, false, &json!({})).unwrap();
    assert!(list["csv"]
        .as_str()
        .unwrap()
        .contains("1,716,397,18,\"Default\",,,\"Side L\""));
}

/// A box mesh of the given size turned `degrees` about Z: 8 corners, 12 triangles.
fn box_mesh(size: [f64; 3], degrees: f64) -> Value {
    let (s, c) = degrees.to_radians().sin_cos();
    let corners: Vec<[f64; 3]> = (0..8)
        .map(|i| {
            let [x, y, z] = [0, 1, 2].map(|k| if i >> k & 1 == 1 { size[k] } else { 0.0 });
            [x * c - y * s, x * s + y * c, z]
        })
        .collect();
    let triangles = [
        [0, 1, 3],
        [0, 3, 2],
        [4, 5, 7],
        [4, 7, 6],
        [0, 1, 5],
        [0, 5, 4],
        [2, 3, 7],
        [2, 7, 6],
        [0, 2, 6],
        [0, 6, 4],
        [1, 3, 7],
        [1, 7, 5],
    ];
    json!({"positions": corners, "triangles": triangles})
}

#[test]
fn a_board_turned_in_its_own_component_is_measured_by_its_smallest_box() {
    // issue 39: a 150 x 800 sketch turned 15 degrees, extruded 18 mm, was 351.9 x 811.6
    let mut size = extents(&box_mesh([150.0, 800.0, 18.0], 15.0));
    size.sort_by(f64::total_cmp);
    for (got, want) in size.iter().zip([18.0, 150.0, 800.0]) {
        assert!((got - want).abs() < 0.1, "{size:?}");
    }
    // square to the axes: the axis sizes, in X, Y, Z order
    assert_eq!(
        extents(&box_mesh([397.0, 18.0, 716.0], 0.0)),
        [397.0, 18.0, 716.0]
    );
    // square to the axes with a corner cut off at 45 degrees: fills under 99 % of its box,
    // but no turned box is smaller, so still the axis sizes in X, Y, Z order
    let outline = [
        [0.0, 0.0],
        [400.0, 0.0],
        [400.0, 500.0],
        [300.0, 600.0],
        [0.0, 600.0],
    ];
    let positions: Vec<[f64; 3]> = [0.0, 18.0]
        .iter()
        .flat_map(|&z| outline.map(|[x, y]| [x, y, z]))
        .collect();
    let mut triangles = vec![];
    for i in 0..5 {
        let j = (i + 1) % 5;
        triangles.extend([[i, j, j + 5], [i, j + 5, i + 5]]);
    }
    for k in 1..4 {
        triangles.extend([[0, k, k + 1], [5, k + 5, k + 6]]);
    }
    let mesh = json!({"positions": positions, "triangles": triangles});
    assert_eq!(extents(&mesh), [400.0, 600.0, 18.0]);
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
    let list = cutlist(&measure(), &BTreeMap::new(), true, &json!({})).unwrap();
    let csv = list["csv"].as_str().unwrap();
    assert!(
        csv.contains("2,30,22,3/4,\"Default\",,,\"Side L, Side R\""),
        "{csv}"
    );
    assert_eq!(list["parts"][1]["length"], "30");
}

#[test]
fn an_inch_list_keeps_the_mm_for_the_cost() {
    let mut measured = measure();
    let worktop =
        json!({"name": "Worktop", "bbox": {"min": [0, 0, 0], "max": [2000.0, 600.0, 38.0]}});
    measured["bodies"].as_array_mut().unwrap().push(worktop);
    let list = cutlist(&measured, &BTreeMap::new(), true, &json!({})).unwrap();
    let side = (list["sheets"].as_array().unwrap().iter())
        .find(|s| s["thickness"] == "3/4")
        .unwrap();
    assert_eq!(side["thickness_mm"], 19.1);
    assert_eq!(list["worktops"][0]["length"], "78 3/4");
    assert_eq!(list["worktops"][0]["length_mm"], 2000.0);
    // in mm the sizes are numbers already
    let list = cutlist(&measured, &BTreeMap::new(), false, &json!({})).unwrap();
    assert!(list["sheets"][0].get("thickness_mm").is_none());
    assert!(list["worktops"][0].get("length_mm").is_none());
}

#[test]
fn the_file_to_save_as() {
    let name = |entry: &str, file: &str| file_name(entry, &json!({"file": file}));
    assert_eq!(name("cutlist", ""), Ok(None));
    assert_eq!(name("cutlist", "  "), Ok(None));
    assert_eq!(file_name("cutlist", &json!({})), Ok(None));
    assert_eq!(name("cabinet", "a"), Ok(None)); // only the cut list saves
    let saved = |file: &str, format: &str| {
        file_name("cutlist", &json!({"file": file, "format": format})).map(Option::unwrap)
    };
    // PDF unless the format says CSV; the format gives the ending, whatever was typed
    assert_eq!(name("cutlist", "kitchen"), Ok(Some("kitchen.pdf".into())));
    assert_eq!(saved("kitchen", "csv"), Ok("kitchen.csv".into()));
    assert_eq!(saved("kitchen.PDF", "csv"), Ok("kitchen.csv".into()));
    assert_eq!(
        saved(" My kitchen 2.csv ", "pdf"),
        Ok("My kitchen 2.pdf".into())
    );
    assert_eq!(saved("kitchen.txt", "pdf"), Ok("kitchen.txt.pdf".into()));
    assert_eq!(saved("kitchen.csv", "labels"), Ok("kitchen.pdf".into()));
    assert_eq!(saved("kitchen.pdf", "dxf"), Ok("kitchen.dxf".into()));
    assert_eq!(saved("kitchen.DXF", "csv"), Ok("kitchen.csv".into()));
    assert!(saved("kitchen", "xlsx").is_err());
    // a name only: the folder is the one the user approved
    for bad in ["../a", "a/b", r"a\b", "C:a", ".hidden"] {
        assert!(name("cutlist", bad).is_err(), "{bad}");
    }
}

/// A cabinet `A` in oak, 400 high and 560 deep, with an MDF divider: bodies as
/// `inspect.measure` has them. `turned` sets a quarter turn (width along Y).
fn cabinet(turned: bool) -> Value {
    let body = |name: &str, material: &str, max: [f64; 3]| {
        let max = if turned {
            [max[1], max[0], max[2]]
        } else {
            max
        };
        json!({"name": name, "material": material, "bbox": {"min": [0, 0, 0], "max": max}})
    };
    json!({"bodies": [
        body("A Side L", "Oak", [18.0, 560.0, 400.0]),
        body("A Side R", "Oak", [18.0, 560.0, 400.0]),
        body("A Divider 1", "MDF", [18.0, 560.0, 400.0]),
        body("A Shelf 1", "Oak", [500.0, 542.0, 18.0]),
        body("A Door", "Oak", [600.0, 18.0, 400.0]),
        body("A Back", "Oak", [564.0, 6.0, 382.0]),
        body("A Plinth", "Oak", [564.0, 18.0, 100.0]),
        body("A Drawer 1 Bottom", "Oak", [500.0, 400.0, 6.0]),
        body("A Drawer 1 Left", "Oak", [12.0, 500.0, 150.0]),
    ]})
}

fn row<'a>(list: &'a Value, part: &str) -> &'a Value {
    let parts = list["parts"].as_array().unwrap();
    parts.iter().find(|p| p["parts"][0] == part).unwrap()
}

#[test]
fn banding_and_grain_by_part() {
    for turned in [false, true] {
        let list = cutlist(&cabinet(turned), &BTreeMap::new(), false, &json!({})).unwrap();
        let size = |part: &str| {
            let r = row(&list, part);
            (
                r["length"].clone(),
                r["width"].clone(),
                r["banding"].clone(),
                r["grain"].clone(),
            )
        };
        let band = |l: u64, w: u64| json!({"length": l, "width": w});
        // oak sides: grain up, so the length is the 400 height; the front edge is a length edge
        assert_eq!(
            size("A Side L"),
            (json!(400.0), json!(560.0), band(1, 0), json!(true))
        );
        // MDF has no grain: the longest side is the length, and the front edge a width edge
        assert_eq!(
            size("A Divider 1"),
            (json!(560.0), json!(400.0), band(0, 1), json!(false))
        );
        // a shelf's grain runs along the cabinet's width, turned or not; its front edge too
        assert_eq!(
            size("A Shelf 1"),
            (json!(500.0), json!(542.0), band(1, 0), json!(true))
        );
        assert_eq!(
            size("A Door"),
            (json!(400.0), json!(600.0), band(2, 2), json!(true))
        );
        assert_eq!(
            size("A Back"),
            (json!(382.0), json!(564.0), band(0, 0), json!(true))
        );
        assert_eq!(
            size("A Plinth"),
            (json!(564.0), json!(100.0), band(0, 0), json!(true))
        );
        assert_eq!(size("A Drawer 1 Bottom").0, json!(500.0), "turned {turned}");
        // a drawer box side runs its grain along the box, not up
        assert_eq!(
            size("A Drawer 1 Left"),
            (json!(500.0), json!(150.0), band(0, 0), json!(true))
        );
        // oak, 18 mm: two sides 400, the shelf 500, the door 2 x 400 + 2 x 600; plus 10 %
        assert_eq!(
            list["banding"],
            json!([
                {"material": "MDF", "thickness": 18.0, "metres": 0.44},
                {"material": "Oak", "thickness": 18.0, "metres": 3.63},
            ])
        );
        assert_eq!(
            (list["size"].clone(), list["banding_mm"].clone()),
            (json!("finished"), json!(1.0))
        );
        let csv = list["csv"].as_str().unwrap();
        assert!(
            csv.contains("1,400,600,18,\"Oak\",yes,L2 W2,\"A Door\""),
            "{csv}"
        );
        assert!(
            csv.contains(
                "\nEdge banding 1 mm,Thickness (mm),Metres with 10 % waste\n\"MDF\",18,0.44\n"
            ),
            "{csv}"
        );
    }
}

#[test]
fn cut_size_takes_off_the_banding() {
    let options = json!({"banding": "2", "size": "cut"});
    let list = cutlist(&cabinet(false), &BTreeMap::new(), false, &options).unwrap();
    let size = |part: &str| {
        (
            row(&list, part)["length"].clone(),
            row(&list, part)["width"].clone(),
        )
    };
    assert_eq!(size("A Door"), (json!(396.0), json!(596.0)));
    assert_eq!(size("A Side L"), (json!(400.0), json!(558.0))); // the front edge runs along the length
    assert_eq!(size("A Back"), (json!(382.0), json!(564.0)));
    assert_eq!(list["size"], "cut");
    assert!(list["csv"]
        .as_str()
        .unwrap()
        .starts_with("Qty,Cut length (mm),Cut width (mm),"));
    // no banding: no marks, nothing to buy
    let list = cutlist(
        &cabinet(false),
        &BTreeMap::new(),
        false,
        &json!({"banding": "none", "size": "cut"}),
    )
    .unwrap();
    assert_eq!(
        row(&list, "A Door")["banding"],
        json!({"length": 0, "width": 0})
    );
    assert_eq!(row(&list, "A Door")["length"], 400.0);
    assert_eq!(list["banding"], json!([]));
    assert!(!list["csv"].as_str().unwrap().contains("Edge banding"));
    for bad in [json!({"banding": "3"}), json!({"size": "raw"})] {
        assert!(
            cutlist(&cabinet(false), &BTreeMap::new(), false, &bad).is_err(),
            "{bad}"
        );
    }
    // imperial: feet beside the metres
    let list = cutlist(&cabinet(false), &BTreeMap::new(), true, &json!({})).unwrap();
    assert_eq!(list["banding"][1]["feet"], 11.909);
}

#[test]
fn a_turned_board_is_banded_in_its_own_axes() {
    // An open door and a turned cabinet's shelf, measured in their own components: X is
    // the cabinet's width there, whichever way the cabinet's Side L stands in the world.
    let local = BTreeMap::from([
        ("A Door".to_string(), [600.0, 18.0, 400.0]),
        ("A Shelf 1".to_string(), [500.0, 542.0, 18.0]),
    ]);
    let list = cutlist(&cabinet(true), &local, false, &json!({})).unwrap();
    assert_eq!(row(&list, "A Door")["length"], 400.0);
    assert_eq!(
        row(&list, "A Door")["banding"],
        json!({"length": 2, "width": 2})
    );
    assert_eq!(row(&list, "A Shelf 1")["length"], 500.0);
    assert_eq!(
        row(&list, "A Shelf 1")["banding"],
        json!({"length": 1, "width": 0})
    );
}

#[test]
fn banding_marks() {
    assert_eq!(mark(2, 2), "L2 W2");
    assert_eq!(mark(1, 0), "L1");
    assert_eq!(mark(0, 1), "W1");
    assert_eq!(mark(0, 0), "");
}

#[test]
fn sheets_from_cutting_layouts() {
    let options =
        json!({"banding": "2", "size": "cut", "kerf": 3, "trim": 5, "sheet": "2500x1250"});
    let list = cutlist(&cabinet(false), &BTreeMap::new(), true, &options).unwrap();
    let layouts = list["layouts"].as_array().unwrap();
    // one sheet per material and thickness: MDF 18, Oak 6, 12 and 18
    let counts: Vec<&Value> = list["sheets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| &s["sheets"])
        .collect();
    assert_eq!(counts, [1, 1, 1, 1]);
    assert_eq!(layouts.len(), 4);
    assert_eq!(
        (&list["kerf_mm"], &list["trim_mm"]),
        (&json!(3.0), &json!(5.0))
    );
    assert_eq!(
        (&list["sheet_mm"], &list["layout_units"]),
        (&json!([2500.0, 1250.0]), &json!("mm"))
    );
    // in mm in an inch design, at the cut size, the grain along the sheet
    let oak = layouts
        .iter()
        .find(|l| l["material"] == "Oak" && l["thickness"] == 18.0)
        .unwrap();
    assert_eq!(oak["size"], json!([2500.0, 1250.0]));
    let door = oak["parts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["part"] == "A Door")
        .unwrap();
    assert_eq!(
        (&door["w"], &door["h"], &door["rotated"]),
        (&json!(396.0), &json!(596.0), &json!(false))
    );
    // Oak 18: 2 x 400 x 558 + 500 x 540 + 396 x 596 + 564 x 100 (no banding on the plinth)
    let used: f64 = 2.0 * 400.0 * 558.0 + 500.0 * 540.0 + 396.0 * 596.0 + 564.0 * 100.0;
    let expected = (used / (2500.0 * 1250.0) * 1000.0).round() / 10.0;
    assert_eq!(oak["yield_pct"], expected);
    let sheet = list["sheets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["material"] == "Oak" && s["thickness"] == "23/32")
        .unwrap();
    assert_eq!(sheet["yield_pct"], expected);
    assert_eq!(list["oversize"], json!([]));

    // a board too big for the sheet is reported with why, and buys no sheet; a worktop is
    // bought to length, not laid out
    let body = |name: &str, material: &str, max: [f64; 3]| json!({"name": name, "material": material, "bbox": {"min": [0, 0, 0], "max": max}});
    let big = json!({"bodies": [
        body("Long", "MDF", [3000.0, 600.0, 18.0]),
        body("Kitchen Worktop", "Oak", [3300.0, 600.0, 38.0]),
    ]});
    let list = cutlist(&big, &BTreeMap::new(), false, &json!({})).unwrap();
    assert_eq!(
        list["oversize"],
        json!([
            {"part": "Long", "material": "MDF", "thickness": 18.0, "length": 3000.0, "width": 600.0,
             "reason": "bigger than the sheet less its trim"},
        ])
    );
    assert_eq!(
        list["worktops"],
        json!([{"part": "Kitchen Worktop", "material": "Oak", "thickness": 38.0, "length": 3300.0, "width": 600.0}])
    );
    let sheets = list["sheets"].as_array().unwrap();
    assert_eq!(sheets.len(), 1, "no sheets for the worktop's 38 mm oak");
    assert!(sheets.iter().all(|s| s["sheets"] == 0) && list["layouts"] == json!([]));
    // an oak door 400 high and 1300 wide fits the sheet only with its grain across it
    let door = json!({"bodies": [
        body("A Side L", "Oak", [18.0, 560.0, 400.0]),
        body("A Door", "Oak", [1300.0, 18.0, 400.0]),
    ]});
    let list = cutlist(&door, &BTreeMap::new(), false, &json!({})).unwrap();
    assert_eq!(
        list["oversize"][0]["reason"],
        "does not fit with its grain along the sheet"
    );

    for bad in [
        json!({"sheet": "2440"}),
        json!({"kerf": -1}),
        json!({"kerf": 11}),
        json!({"kerf": "4"}),
        json!({"trim": 51}),
        json!({"diagrams": "maybe"}),
    ] {
        let error = cutlist(&cabinet(false), &BTreeMap::new(), false, &bad).unwrap_err();
        let key = bad.as_object().unwrap().keys().next().unwrap().clone();
        assert!(error.starts_with(&key), "{error}");
    }
}

#[test]
fn a_worktop_made_by_worktop_is_known_whatever_its_name() {
    let top = json!({"name": "Island top", "material": "Oak", "bbox": {"min": [0, 0, 0], "max": [2400.0, 900.0, 38.0]}});
    let measured = json!({"bodies": [top]});
    let laid = cutlist(&measured, &BTreeMap::new(), false, &json!({})).unwrap();
    assert_eq!(
        laid["worktops"],
        json!([]),
        "not a worktop by its name alone"
    );
    let options = json!({"_worktops": ["Island top"]});
    let list = cutlist(&measured, &BTreeMap::new(), false, &options).unwrap();
    assert_eq!(
        list["worktops"],
        json!([{"part": "Island top", "material": "Oak", "thickness": 38.0, "length": 2400.0, "width": 900.0}])
    );
    assert!(list["sheets"]
        .as_array()
        .unwrap()
        .iter()
        .all(|s| s["sheets"] == 0));
    assert_eq!(list["layouts"], json!([]));
    // bought with finished edges: no banding
    let band = json!({"length": 0, "width": 0});
    assert_eq!(row(&list, "Island top")["banding"], band);
    assert_eq!(list["banding"], json!([]));
}

#[test]
fn a_rail_is_banded_at_the_front_like_a_shelf() {
    for turned in [false, true] {
        let mut measured = cabinet(turned);
        let max = if turned {
            [300.0, 564.0, 18.0]
        } else {
            [564.0, 300.0, 18.0]
        };
        let rail =
            json!({"name": "A Rail", "material": "Oak", "bbox": {"min": [0, 0, 0], "max": max}});
        measured["bodies"].as_array_mut().unwrap().push(rail);
        let list = cutlist(&measured, &BTreeMap::new(), false, &json!({})).unwrap();
        let r = row(&list, "A Rail");
        // grain along the cabinet's width, the front edge (along the grain) banded
        assert_eq!(
            (r["length"].clone(), r["width"].clone()),
            (json!(564.0), json!(300.0))
        );
        assert_eq!(
            r["banding"],
            json!({"length": 1, "width": 0}),
            "turned {turned}"
        );
        assert_eq!(r["grain"], json!(true));
    }
}

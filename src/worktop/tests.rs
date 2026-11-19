use super::*;

fn body(name: &str, min: [f64; 3], max: [f64; 3]) -> Value {
    json!({"name": name, "bbox": {"min": min, "max": max}})
}

/// A box cabinet facing -Y, its back at y = `back`: Side L, Side R and Back.
fn cabinet(name: &str, x: f64, width: f64, back: f64, floor: f64, height: f64) -> Vec<Value> {
    let (y, z) = (back - 560.0, floor + height);
    vec![
        body(
            &format!("{name} Side L"),
            [x, y, floor],
            [x + 18.0, back, z],
        ),
        body(
            &format!("{name} Side R"),
            [x + width - 18.0, y, floor],
            [x + width, back, z],
        ),
        body(
            &format!("{name} Back"),
            [x, back - 6.0, floor],
            [x + width, back, z],
        ),
    ]
}

/// A corner cabinet for a right corner, 1000 x 1000, its back at y = 0 and the right wall at
/// x = `x` + 1000.
fn corner(name: &str, x: f64) -> Vec<Value> {
    let (w, h) = (1000.0, 720.0);
    vec![
        body(
            &format!("{name} Side L"),
            [x, -560.0, 0.0],
            [x + 18.0, 0.0, h],
        ),
        body(
            &format!("{name} Side R"),
            [x + w - 560.0, -1000.0, 0.0],
            [x + w, -982.0, h],
        ),
        body(&format!("{name} Back"), [x, -6.0, 0.0], [x + w, 0.0, h]),
        body(
            &format!("{name} Back return"),
            [x + w - 6.0, -1000.0, 0.0],
            [x + w, 0.0, h],
        ),
    ]
}

fn measure(cabinets: Vec<Vec<Value>>) -> Value {
    json!({"bodies": cabinets.concat()})
}

fn design() -> Value {
    let p = |name: &str, value: f64| json!({"name": name, "value": value});
    json!({"params": [
        p("A_width", 600.0), p("B_width", 800.0), p("C_width", 400.0), p("K_width", 1000.0),
        p("K_return", 1000.0), p("A_depth", 560.0), p("B_depth", 580.0), p("C_depth", 560.0),
        p("K_depth", 560.0), p("A_height", 720.0), p("B_height", 720.0), p("C_height", 720.0),
        p("K_height", 720.0),
    ]})
}

fn picked(names: &[&str]) -> Vec<String> {
    names.iter().map(|n| n.to_string()).collect()
}

fn named<'a>(commands: &'a Commands, name: &str) -> &'a Value {
    &commands.iter().find(|c| c.1["name"] == name).unwrap().1
}

#[test]
fn a_run_left_to_right_gets_one_board_over_it() {
    // picked in any order, several boards of one cabinet; the corner's width leg is along the run
    let m = measure(vec![
        cabinet("B", 600.0, 800.0, 0.0, 0.0, 720.0),
        cabinet("A", 0.0, 600.0, 0.0, 0.0, 720.0),
        corner("K", 1400.0),
    ]);
    let p = json!({"name": "Worktop", "left_overhang": 10, "right_overhang": 0});
    let at = picked(&["B Side L", "K Top", "A Side R", "B Back"]);
    let (commands, summary) = build(&p, &at, &m, &design()).unwrap();
    assert_eq!(
        named(&commands, "Worktop_width")["expression"],
        "A_width + B_width + K_width + 10 mm"
    );
    // the deepest cabinet, and the front overhang
    assert_eq!(
        named(&commands, "Worktop_depth")["expression"],
        "B_depth + 40 mm"
    );
    assert_eq!(named(&commands, "Worktop_thickness")["expression"], "38 mm");
    let board = named(&commands, "Worktop");
    assert_eq!(board["body_name"], "Worktop");
    assert_eq!(
        board["corner"],
        json!(["-(10 mm)", "-Worktop_depth", "A_height"])
    );
    assert_eq!(board["length"], "Worktop_width");
    assert_eq!(summary["at"], json!([0.0, 0.0, 0.0]));
    assert_eq!(summary["turn"], 0);
    assert_eq!(summary["cabinets"], json!(["A", "B", "K"]));
    assert_eq!(summary["size"]["length"], 2410.0);
    assert_eq!(summary["size"]["depth"], 620.0);
    let group = &commands.last().unwrap();
    assert_eq!(group.0, "timeline.group");
    assert_eq!(group.1["features"], json!(["Worktop"]));
}

#[test]
fn a_run_along_a_corner_cabinets_return_is_turned() {
    // A cabinet turned -90 at the end of the corner's return: the run goes -Y, its back is
    // the right wall, and the corner's return lies along it.
    let x = 1000.0; // the right wall
    let turned = vec![
        body("C Side L", [x - 560.0, -1018.0, 0.0], [x, -1000.0, 720.0]),
        body("C Side R", [x - 560.0, -1400.0, 0.0], [x, -1382.0, 720.0]),
        body("C Back", [x - 6.0, -1400.0, 0.0], [x, -1000.0, 720.0]),
    ];
    let m = measure(vec![corner("K", 0.0), turned]);
    let p = json!({"name": "Top"});
    let (commands, summary) = build(&p, &picked(&["C Top", "K Door"]), &m, &design()).unwrap();
    assert_eq!(
        named(&commands, "Top_width")["expression"],
        "K_return + C_width"
    );
    // its back left corner, seen from its front: at the back wall, on the right wall
    assert_eq!(summary["at"], json!([1000.0, 0.0, 0.0]));
    assert_eq!(summary["turn"], -90);
    // the corner alone runs along its back wall leg
    let (commands, summary) = build(&p, &picked(&["K Top"]), &m, &design()).unwrap();
    assert_eq!(named(&commands, "Top_width")["expression"], "K_width");
    assert_eq!(
        (summary["turn"].clone(), summary["at"].clone()),
        (json!(0), json!([0.0, 0.0, 0.0]))
    );
}

#[test]
fn anything_but_one_straight_run_is_refused() {
    let try_run = |cabinets: Vec<Vec<Value>>, at: &[&str]| {
        let p = json!({"name": "Worktop"});
        build(&p, &picked(at), &measure(cabinets), &design()).unwrap_err()
    };
    let a = || cabinet("A", 0.0, 600.0, 0.0, 0.0, 720.0);
    let gap = try_run(
        vec![a(), cabinet("B", 610.0, 800.0, 0.0, 0.0, 720.0)],
        &["A Top", "B Top"],
    );
    assert_eq!(
        gap,
        "the selected cabinets are not one straight run: there is a gap between `A` and `B`"
    );
    let overlap = try_run(
        vec![a(), cabinet("B", 500.0, 800.0, 0.0, 0.0, 720.0)],
        &["A Top", "B Top"],
    );
    assert!(overlap.ends_with("`A` and `B` overlap"), "{overlap}");
    let above = try_run(
        vec![a(), cabinet("B", 0.0, 600.0, 0.0, 720.0, 720.0)],
        &["A Top", "B Top"],
    );
    assert!(
        above.contains("does not stand on the same floor"),
        "{above}"
    );
    let higher = try_run(
        vec![a(), cabinet("B", 600.0, 800.0, 0.0, 0.0, 900.0)],
        &["A Top", "B Top"],
    );
    assert!(higher.contains("is not as high as"), "{higher}");
    let back = try_run(
        vec![a(), cabinet("B", 600.0, 800.0, -20.0, 0.0, 720.0)],
        &["A Top", "B Top"],
    );
    assert!(back.contains("are not flush"), "{back}");
    // a cabinet turned to face another way
    let turned = vec![
        body("B Side L", [600.0, -18.0, 0.0], [1160.0, 0.0, 720.0]),
        body("B Side R", [600.0, -600.0, 0.0], [1160.0, -582.0, 720.0]),
    ];
    let other_way = try_run(vec![a(), turned], &["A Top", "B Top"]);
    assert!(other_way.ends_with("`B` faces another way"), "{other_way}");
    // a corner cabinet in the middle
    let middle = try_run(
        vec![
            cabinet("A", -600.0, 600.0, 0.0, 0.0, 720.0),
            corner("K", 0.0),
            cabinet("B", 1000.0, 800.0, 0.0, 0.0, 720.0),
        ],
        &["A Top", "K Top", "B Top"],
    );
    assert!(middle.ends_with("`K` is not at an end"), "{middle}");
    // something that is not a cabinet, and nothing
    assert_eq!(
        try_run(vec![a()], &["Worktop"]),
        "`Worktop` is not part of a cabinet"
    );
    assert_eq!(
        try_run(vec![a()], &[]),
        "select the cabinets for the worktop first"
    );
}

#[test]
fn an_inch_worktop_keeps_inches_and_a_cabinets_name_is_refused() {
    let m = measure(vec![cabinet("A", 0.0, 600.0, 0.0, 0.0, 720.0)]);
    let at = picked(&["A Top"]);
    let p = json!({"name": "W", "units": "in", "thickness": 1.5, "front_overhang": 1, "left_overhang": 0.5});
    let (commands, summary) = build(&p, &at, &m, &design()).unwrap();
    assert_eq!(named(&commands, "W_thickness")["expression"], "1.5 in");
    assert_eq!(named(&commands, "W_depth")["expression"], "A_depth + 1 in");
    assert_eq!(
        named(&commands, "W_width")["expression"],
        "A_width + 0.5 in"
    );
    assert_eq!(named(&commands, "W")["corner"][0], "-(0.5 in)");
    assert!((summary["size"]["thickness"].as_f64().unwrap() - 38.1).abs() < 1e-9);
    // a body called "A top" would be taken for a part of cabinet A
    let p = json!({"name": "A top"});
    let refused = build(&p, &at, &m, &design()).unwrap_err();
    assert_eq!(
        refused,
        "name: `A top` would read as a part of the cabinet `A`; choose another"
    );
    assert!(build(&json!({"name": "W", "thickness": -1}), &at, &m, &design()).is_err());
}

#[test]
fn the_worktop_holds_where_the_run_holds() {
    let m = measure(vec![
        cabinet("A", 0.0, 600.0, 0.0, 0.0, 720.0),
        cabinet("B", 600.0, 800.0, 0.0, 0.0, 720.0),
        cabinet("C", 1400.0, 400.0, 0.0, 0.0, 720.0),
    ]);
    let at = picked(&["A Top", "B Top", "C Top"]);
    let make = |held: Value| {
        let p = json!({"name": "W", "left_overhang": 10, "_held": held});
        let (commands, summary) = build(&p, &at, &m, &design()).unwrap();
        (
            named(&commands, "W")["corner"].clone(),
            summary["at"].clone(),
        )
    };
    // B grounded: on its middle, which stays put when it grows; A and C hang on it
    let (board, at_) =
        make(json!({"B": {"grounded": true}, "A": {"link": {"to": "B", "place": "left"}}}));
    assert_eq!(
        board,
        json!([
            "-(B_width / 2 + A_width + 10 mm)",
            "B_depth / 2 - W_depth",
            "B_height"
        ])
    );
    assert_eq!(at_, json!([1000.0, -280.0, 0.0]));
    // C held left of a cabinet outside the run: its right side stays put, the run grows left
    let (board, at_) = make(
        json!({"C": {"link": {"to": "D", "place": "left"}}, "B": {"link": {"to": "C", "place": "left"}}}),
    );
    assert_eq!(
        board,
        json!([
            "-(C_width + A_width + B_width + 10 mm)",
            "-W_depth",
            "C_height"
        ])
    );
    assert_eq!(at_, json!([1800.0, 0.0, 0.0]));
    // A held right of a cabinet outside: its left side, as without anything
    let (board, at_) = make(json!({"A": {"link": {"to": "Tall", "place": "right"}}}));
    assert_eq!(board, json!(["-(10 mm)", "-W_depth", "A_height"]));
    assert_eq!(at_, json!([0.0, 0.0, 0.0]));
    // a grounded corner cabinet: on its middle, its return across the run
    let m = measure(vec![
        cabinet("A", 0.0, 600.0, 0.0, 0.0, 720.0),
        corner("K", 600.0),
    ]);
    let p = json!({"name": "W", "_held": {"K": {"grounded": true}}});
    let (commands, summary) = build(&p, &picked(&["A Top", "K Top"]), &m, &design()).unwrap();
    assert_eq!(
        named(&commands, "W")["corner"],
        json!([
            "-(K_width / 2 + A_width)",
            "K_return / 2 - W_depth",
            "K_height"
        ])
    );
    assert_eq!(summary["at"], json!([1100.0, -500.0, 0.0]));
}

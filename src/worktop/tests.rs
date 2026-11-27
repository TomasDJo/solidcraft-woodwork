use super::*;

fn body(name: &str, min: [f64; 3], max: [f64; 3]) -> Value {
    json!({"name": name, "bbox": {"min": min, "max": max}})
}

/// A box cabinet against the back wall at y = 0, facing -Y, 560 deep, from x to x + width.
fn base(name: &str, x: f64, width: f64, height: f64) -> Vec<Value> {
    let side = |s: &str, x0: f64| {
        body(
            &format!("{name} {s}"),
            [x0, -560.0, 0.0],
            [x0 + 18.0, 0.0, height],
        )
    };
    vec![
        side("Side L", x),
        side("Side R", x + width - 18.0),
        body(
            &format!("{name} Back"),
            [x, -6.0, 0.0],
            [x + width, 0.0, height],
        ),
        body(
            &format!("{name} Door"),
            [x, -580.0, 0.0],
            [x + width, -560.0, height],
        ),
    ]
}

/// A box cabinet against the left wall at x = 0, facing +X, from y - width to y.
fn left_wall(name: &str, y: f64, width: f64) -> Vec<Value> {
    let side = |s: &str, y0: f64| {
        body(
            &format!("{name} {s}"),
            [0.0, y0, 0.0],
            [560.0, y0 + 18.0, 720.0],
        )
    };
    vec![
        side("Side L", y - width),
        side("Side R", y - 18.0),
        body(
            &format!("{name} Back"),
            [0.0, y - width, 0.0],
            [6.0, y, 720.0],
        ),
    ]
}

/// A corner cabinet for a left corner (the walls at x = 0 and y = 0), 1000 along the back wall
/// and `ret` along the left one; its return's end panel `end` long (a diagonal one's differs).
fn corner(name: &str, ret: f64, end: f64) -> Vec<Value> {
    vec![
        body(
            &format!("{name} Side L"),
            [0.0, -ret, 0.0],
            [end, -ret + 18.0, 720.0],
        ),
        body(
            &format!("{name} Side R"),
            [982.0, -560.0, 0.0],
            [1000.0, 0.0, 720.0],
        ),
        body(
            &format!("{name} Back"),
            [0.0, -6.0, 0.0],
            [1000.0, 0.0, 720.0],
        ),
        body(
            &format!("{name} Back return"),
            [0.0, -ret, 0.0],
            [6.0, 0.0, 720.0],
        ),
    ]
}

fn measure(cabinets: Vec<Vec<Value>>) -> Value {
    json!({"bodies": cabinets.concat()})
}

fn names(names: &[&str]) -> Vec<String> {
    names.iter().map(|n| n.to_string()).collect()
}

fn named<'a>(commands: &'a Commands, name: &str) -> &'a Value {
    &commands.iter().find(|c| c.1["name"] == name).unwrap().1
}

fn refused(p: Value, cabinets: &[&str], m: &Value) -> String {
    build(&p, &names(cabinets), m).err().unwrap()
}

/// The kitchen corner: K in the left corner, A and (after a 600 gap) B along the back wall,
/// C along the left wall.
fn kitchen() -> Value {
    measure(vec![
        corner("K", 1000.0, 560.0),
        base("A", 1000.0, 600.0, 720.0),
        base("B", 2200.0, 600.0, 720.0),
        left_wall("C", -1000.0, 600.0),
    ])
}

#[test]
fn a_straight_run_with_a_gap_is_one_piece() {
    let m = measure(vec![
        base("A", 0.0, 600.0, 720.0),
        base("B", 1200.0, 600.0, 720.0),
    ]);
    let p = json!({"left_overhang": 10, "right_overhang": 20});
    let (commands, summary) = build(&p, &names(&["B", "A"]), &m).unwrap();
    // spanning the gap, the back on the wall, 40 past the carcass front, on top
    let top = named(&commands, "Worktop");
    assert_eq!(top["corner"], json!([-10.0, -600.0, 720.0]));
    assert_eq!(
        [&top["length"], &top["width"], &top["height"]],
        [&json!("1830 mm"), &json!("600 mm"), &json!("38 mm")]
    );
    assert_eq!(top["operation"], "new");
    assert_eq!(summary["bodies"], json!(["Worktop"]));
    assert_eq!(summary["pieces"][0]["length"], 1830.0);
    // one history folder
    let group = &commands.last().unwrap().1;
    assert_eq!(group["features"], json!(["Worktop"]));
}

#[test]
fn inches() {
    let m = measure(vec![base("A", 0.0, 600.0, 720.0)]);
    let p = json!({"units": "in", "thickness": 1.5, "front_overhang": 1, "name": "Top"});
    let (commands, _) = build(&p, &names(&["A"]), &m).unwrap();
    let top = named(&commands, "Top");
    assert_eq!(top["height"], "38.1 mm");
    assert_eq!(top["width"], "585.4 mm");
}

#[test]
fn an_l_with_a_butt_joint() {
    let p = json!({"left_overhang": 10, "right_overhang": 20});
    let (commands, summary) = build(&p, &names(&["A", "B", "C", "K"]), &kitchen()).unwrap();
    // the back run is longer: it passes through to the left wall, and is piece 1
    let one = named(&commands, "Worktop 1");
    assert_eq!(one["corner"], json!([0.0, -600.0, 720.0]));
    assert_eq!(one["length"], "2820 mm");
    // the left run stops at its front edge, and overhangs at its free end, its left
    let two = named(&commands, "Worktop 2");
    assert_eq!(two["corner"], json!([0.0, -1610.0, 720.0]));
    assert_eq!(
        [&two["length"], &two["width"]],
        [&json!("600 mm"), &json!("1010 mm")]
    );
    assert_eq!(summary["bodies"], json!(["Worktop 1", "Worktop 2"]));
    assert_eq!(summary["joint"], "butt");
}

#[test]
fn an_l_with_a_mitre() {
    let p = json!({"joint": "mitre"});
    let (commands, summary) = build(&p, &names(&["K", "A", "B", "C"]), &kitchen()).unwrap();
    // both to the walls, cut from the inner corner (600, -600) to the outer one (0, 0)
    let lines: Vec<&Value> = (commands.iter())
        .filter(|c| c.0 == "sketch.line")
        .map(|c| &c.1["points"])
        .collect();
    assert_eq!(
        lines,
        [
            &json!([[0.0, 0.0], [2800.0, 0.0], [2800.0, -600.0], [600.0, -600.0]]),
            &json!([
                [0.0, -1600.0],
                [0.0, 0.0],
                [600.0, -600.0],
                [600.0, -1600.0]
            ])
        ]
    );
    let one = named(&commands, "Worktop 1");
    assert_eq!(one["sketch"], "Worktop 1 outline");
    assert_eq!(one["distance"], "38 mm");
    assert_eq!(one["operation"], "new");
    // bought at its whole length
    assert_eq!(summary["pieces"][0]["length"], 2800.0);
    assert_eq!(summary["pieces"][1]["length"], 1600.0);
    let group = &commands.last().unwrap().1;
    assert_eq!(
        group["features"],
        json!([
            "Worktop 1 outline",
            "Worktop 1",
            "Worktop 2 outline",
            "Worktop 2"
        ])
    );
}

#[test]
fn a_mitre_between_unequal_depths() {
    // C 400 deep: the cut goes from (0, 0) to where the two front edges meet, (440, -600)
    let mut c = left_wall("C", -1000.0, 600.0);
    for b in &mut c {
        b["bbox"]["max"][0] = json!(b["bbox"]["max"][0].as_f64().unwrap().min(400.0));
    }
    let m = measure(vec![base("A", 0.0, 1000.0, 720.0), c]);
    let p = json!({"joint": "mitre"});
    let (pieces, _) = plan(&m, &names(&["A", "C"]), "mitre", [40.0, 0.0, 0.0]).unwrap();
    assert_eq!(pieces[0].outline()[3], [440.0, -600.0]);
    assert_eq!(pieces[1].outline()[2], [440.0, -600.0]);
    assert!(build(&p, &names(&["A", "C"]), &m).is_ok());
}

#[test]
fn a_diagonal_corner_cabinet_gets_the_same_square_l() {
    // its return's end is 460 deep: that leg is as deep as its end panel
    let m = measure(vec![corner("K", 900.0, 460.0)]);
    let (pieces, top) = plan(&m, &names(&["K"]), "butt", [40.0, 0.0, 0.0]).unwrap();
    assert_eq!(top, 720.0);
    assert_eq!(
        pieces[0].outline(),
        [[0.0, 0.0], [1000.0, 0.0], [1000.0, -600.0], [0.0, -600.0]]
    );
    assert_eq!(
        pieces[1].outline(),
        [
            [0.0, -900.0],
            [0.0, -600.0],
            [500.0, -600.0],
            [500.0, -900.0]
        ]
    );
}

#[test]
fn a_u() {
    // a right corner cabinet mirrored at x = 3000
    let mut right = corner("R", 1000.0, 560.0);
    for b in &mut right {
        let (lo, hi) = (
            b["bbox"]["min"][0].as_f64().unwrap(),
            b["bbox"]["max"][0].as_f64().unwrap(),
        );
        b["bbox"]["min"][0] = json!(3000.0 - hi);
        b["bbox"]["max"][0] = json!(3000.0 - lo);
    }
    let m = measure(vec![
        corner("K", 1000.0, 560.0),
        base("A", 1000.0, 1000.0, 720.0),
        right,
    ]);
    let (pieces, _) = plan(&m, &names(&["K", "A", "R"]), "butt", [40.0, 0.0, 0.0]).unwrap();
    assert_eq!(pieces.len(), 3);
    assert_eq!(pieces[0].outline()[0], [0.0, 0.0]);
    assert_eq!(pieces[0].outline()[1], [3000.0, 0.0]);
    // both returns stop at the back piece's front edge
    assert_eq!(pieces[1].outline()[1][1], -600.0);
    assert_eq!(pieces[2].outline()[0][1], -600.0);
}

#[test]
fn refusals() {
    let m = kitchen();
    let high = measure(vec![
        base("A", 0.0, 600.0, 720.0),
        base("H", 600.0, 600.0, 900.0),
    ]);
    assert!(refused(json!({}), &["A", "H"], &high).contains("`H` is not as high as `A`"));
    // facing each other across the room
    // turned half round: x and y mirrored
    let mut facing = base("F", 0.0, 600.0, 720.0);
    for b in &mut facing {
        for (i, about) in [(0, 600.0), (1, -3000.0)] {
            let (lo, hi) = (
                b["bbox"]["min"][i].as_f64().unwrap(),
                b["bbox"]["max"][i].as_f64().unwrap(),
            );
            b["bbox"]["min"][i] = json!(about - hi);
            b["bbox"]["max"][i] = json!(about - lo);
        }
    }
    let galley = measure(vec![base("A", 0.0, 600.0, 720.0), facing.clone()]);
    let why = refused(json!({}), &["A", "F"], &galley);
    assert!(
        why.contains("face opposite ways: make one worktop per run"),
        "{why}"
    );
    // the same way, against another wall
    let mut ahead = base("G", 1000.0, 600.0, 720.0);
    for b in &mut ahead {
        b["bbox"]["min"][1] = json!(b["bbox"]["min"][1].as_f64().unwrap() - 1000.0);
        b["bbox"]["max"][1] = json!(b["bbox"]["max"][1].as_f64().unwrap() - 1000.0);
    }
    let two = measure(vec![base("A", 0.0, 600.0, 720.0), ahead]);
    let why = refused(json!({}), &["A", "G"], &two);
    assert!(why.contains("do not stand against the same wall"), "{why}");
    // a run whose wall is in the middle of the back run, not at its end
    let mut middle = left_wall("C", -1000.0, 600.0);
    for b in &mut middle {
        for end in ["min", "max"] {
            b["bbox"][end][0] = json!(b["bbox"][end][0].as_f64().unwrap() + 1000.0);
        }
    }
    let apart = measure(vec![base("A", 0.0, 2000.0, 720.0), middle]);
    let why = refused(json!({}), &["A", "C"], &apart);
    assert!(why.contains("do not meet in a corner"), "{why}");
    // turned off a quarter turn: a side does not fill its box
    let mut turned = base("T", 0.0, 600.0, 720.0);
    turned[0]["volume_mm3"] = json!(1000.0);
    let why = refused(json!({}), &["T"], &measure(vec![turned]));
    assert!(why.contains("turned off a quarter turn"), "{why}");
    let why = refused(json!({"name": "A Top"}), &["A"], &m);
    assert!(
        why.contains("would read as a part of the cabinet `A`"),
        "{why}"
    );
    assert!(refused(json!({}), &[], &m).contains("select the cabinets"));
    assert!(refused(json!({"joint": "dovetail"}), &["A"], &m).contains("joint must be"));
    assert!(refused(json!({"thickness": 0}), &["A"], &m).contains("thickness"));
    // overhangs that eat a piece
    let why = refused(json!({"left_overhang": -700}), &["A"], &m);
    assert!(why.contains("must not be negative"), "{why}");
}

#[test]
fn dog_hole_grid() {
    // 1000 long: centres 70 in at least, 96 apart, centred
    assert_eq!(grid(0.0, 1000.0, 70.0), Some((9, 116.0)));
    assert_eq!(grid(0.0, 140.0, 70.0), Some((1, 70.0)));
    assert_eq!(grid(0.0, 139.0, 70.0), None);
    assert_eq!(grid(100.0, 300.0, 10.0), Some((2, 152.0)));
}

#[test]
fn dog_holes_all_over_the_first_piece() {
    let m = measure(vec![base("A", 0.0, 1000.0, 720.0)]);
    let p = json!({"dog_holes": "all", "front_overhang": 40});
    let (commands, summary) = build(&p, &names(&["A"]), &m).unwrap();
    // 9 along, 5 across (600 deep: room for 460 between the margins), the first at the front left
    let hole = named(&commands, "Worktop dog hole");
    assert_eq!(hole["base"], json!([116.0, -492.0, 720.0]));
    assert_eq!(
        [&hole["diameter"], &hole["operation"]],
        [&json!("20 mm"), &json!("cut")]
    );
    assert_eq!(hole["targets"], json!(["Worktop"]));
    let pattern = named(&commands, "Worktop dog holes");
    assert_eq!(pattern["features"], json!(["Worktop dog hole"]));
    assert_eq!(
        [&pattern["count1"], &pattern["count2"]],
        [&json!(9), &json!(5)]
    );
    assert_eq!(
        [&pattern["dir1"], &pattern["dir2"]],
        [&json!([1.0, 0.0, 0]), &json!([0.0, 1.0, 0])]
    );
    assert_eq!(pattern["spacing1"], "96 mm");
    assert_eq!(summary["dog_holes"], 45);
}

#[test]
fn dog_holes_in_a_zone() {
    let m = measure(vec![base("A", 0.0, 1000.0, 720.0)]);
    let zone = json!({"dog_holes": "zone", "zone_x": 100, "zone_y": 100, "zone_length": 200, "zone_width": 20});
    let (commands, summary) = build(&zone, &names(&["A"]), &m).unwrap();
    // two holes along, one row: a pattern one way only
    assert_eq!(
        named(&commands, "Worktop dog hole")["base"],
        json!([152.0, -490.0, 720.0])
    );
    let pattern = named(&commands, "Worktop dog holes");
    assert_eq!(pattern["count1"], 2);
    assert!(pattern.get("dir2").is_none());
    assert_eq!(summary["dog_holes"], 2);
    let too_big = json!({"dog_holes": "zone", "zone_x": 500, "zone_length": 600});
    assert!(refused(too_big, &["A"], &m).contains("zone does not fit"));
    let tiny = json!({"dog_holes": "zone", "zone_length": 10});
    assert!(refused(tiny, &["A"], &m).contains("no room for a dog hole"));
}

#[test]
fn a_rebated_cut_out() {
    let m = measure(vec![base("A", 0.0, 1000.0, 720.0)]);
    let p = json!({"cutout": "rebated", "cutout_length": 300, "cutout_width": 200, "cutout_x": 500, "cutout_y": 300, "rebate_width": 10, "rebate_depth": 8});
    let (commands, _) = build(&p, &names(&["A"]), &m).unwrap();
    let rebate = named(&commands, "Worktop rebate");
    assert_eq!(rebate["corner"], json!([340.0, -410.0, 750.0]));
    assert_eq!(
        [&rebate["length"], &rebate["width"], &rebate["height"]],
        [&json!("320 mm"), &json!("220 mm"), &json!("8 mm")]
    );
    let opening = named(&commands, "Worktop cut-out");
    assert_eq!(opening["corner"], json!([350.0, -400.0, 720.0]));
    assert_eq!(
        [&opening["length"], &opening["height"]],
        [&json!("300 mm"), &json!("38 mm")]
    );
    assert_eq!(opening["targets"], json!(["Worktop"]));
    // a plain one has no rebate
    let plain = json!({"cutout": "plain"});
    let (commands, _) = build(&plain, &names(&["A"]), &m).unwrap();
    assert!(!commands.iter().any(|c| c.1["name"] == "Worktop rebate"));
    // too near the edge with its rebate, or too deep a rebate
    let off = json!({"cutout": "rebated", "cutout_y": 105, "cutout_width": 200});
    assert!(refused(off, &["A"], &m).contains("cut-out does not fit"));
    assert!(
        refused(json!({"cutout": "plain", "cutout_x": 900}), &["A"], &m).contains("does not fit")
    );
    assert!(
        refused(json!({"cutout": "rebated", "rebate_depth": 38}), &["A"], &m).contains("rebate")
    );
}

#[test]
fn a_cut_out_must_miss_a_mitred_end() {
    // piece 1 of the mitred L is cut away in front of x = 600 at the corner
    let p = json!({"joint": "mitre", "cutout": "plain", "cutout_length": 200, "cutout_width": 200, "cutout_x": 150, "cutout_y": 150});
    assert!(refused(p, &["K", "A", "B", "C"], &kitchen()).contains("does not fit"));
    let p = json!({"joint": "mitre", "cutout": "plain", "cutout_length": 200, "cutout_width": 200, "cutout_x": 750, "cutout_y": 150});
    assert!(build(&p, &names(&["K", "A", "B", "C"]), &kitchen()).is_ok());
}

#[test]
fn pieces_of_a_worktop() {
    assert!(piece_of("Worktop", "Worktop"));
    assert!(piece_of("Worktop 2", "Worktop"));
    assert!(!piece_of("Worktop return", "Worktop"));
    assert!(!piece_of("Worktop ", "Worktop"));
    assert!(!piece_of("Worktops", "Worktop"));
    // its tool's parts go with it, so a run again does not take them for a name clash
    assert!(piece_of("Worktop Tool shelf", "Worktop"));
    assert!(piece_of("Worktop Tool batten R", "Worktop"));
    assert!(!piece_of("Worktop Tool", "Worktop"));
}

#[test]
fn a_piece_belongs_to_the_longest_name_first() {
    let components = ["Worktop", "Base 1", "Worktop 2"];
    assert_eq!(owners("Worktop 2", &components), ["Worktop 2", "Worktop"]);
    assert_eq!(owners("Worktop 1", &components), ["Worktop"]);
    assert!(owners("Base 1 Top", &components).is_empty());
}

const TOOL: &str = r#"{"tool": "benchtop", "tool_width": 640, "tool_depth": 540, "tool_height": 300, "tool_x": 600, "tool_y": 320, "tool_clearance": 3}"#;

fn tool(more: Value) -> Value {
    let mut p: Value = serde_json::from_str(TOOL).unwrap();
    for (k, v) in more.as_object().unwrap() {
        p[k] = v.clone();
    }
    p
}

#[test]
fn a_benchtop_tool_on_a_shelf_in_the_cabinet_below() {
    // A from x 0 to 1000, sides 18, back 6 thick at the wall, no bottom: the floor is z 0
    let m = measure(vec![base("A", 0.0, 1000.0, 720.0)]);
    let (commands, summary) = build(&tool(json!({})), &names(&["A"]), &m).unwrap();
    // the opening: 646 x 546 through the top, its middle 600 in and 320 from the front edge,
    // behind the door
    let opening = named(&commands, "Worktop tool opening");
    assert_eq!(opening["corner"], json!([277.0, -553.0, 720.0]));
    assert_eq!(
        [&opening["length"], &opening["width"], &opening["height"]],
        [&json!("646 mm"), &json!("546 mm"), &json!("38 mm")]
    );
    assert_eq!(opening["targets"], json!(["Worktop"]));
    // the shelf between the sides' inner faces, from the back to the sides' front, its top
    // 300 under the top surface (758)
    let shelf = named(&commands, "Worktop Tool shelf");
    assert_eq!(shelf["corner"], json!([18.0, -560.0, 440.0]));
    assert_eq!(
        [&shelf["length"], &shelf["width"], &shelf["height"]],
        [&json!("964 mm"), &json!("554 mm"), &json!("18 mm")]
    );
    assert_eq!(shelf["body_name"], "Worktop Tool shelf");
    // a batten under it against each side
    let l = named(&commands, "Worktop Tool batten L");
    let r = named(&commands, "Worktop Tool batten R");
    assert_eq!(l["corner"], json!([18.0, -560.0, 400.0]));
    assert_eq!(r["corner"], json!([964.0, -560.0, 400.0]));
    assert_eq!(
        [&r["length"], &r["width"], &r["height"]],
        [&json!("18 mm"), &json!("554 mm"), &json!("40 mm")]
    );
    assert_eq!(
        summary["bodies"],
        json!([
            "Worktop",
            "Worktop Tool shelf",
            "Worktop Tool batten L",
            "Worktop Tool batten R"
        ])
    );
    assert_eq!(summary["tool"]["cabinet"], "A");
    assert_eq!(summary["tool"]["shelf_z"], 458.0);
    // no top in the test cabinet: nothing of it to cut; the tool's space for lib.rs
    assert_eq!(summary["tool"]["cut"]["targets"], json!([]));
    assert_eq!(summary["tool"]["cut"]["min"], json!([277.0, -553.0, 458.0]));
    assert_eq!(summary["tool"]["cut"]["max"], json!([923.0, -7.0, 720.0]));
    // in its history folder
    let group = &commands.last().unwrap().1;
    assert!(group["features"]
        .as_array()
        .unwrap()
        .contains(&json!("Worktop Tool shelf")));
    // no bolt spacing: no bolt holes
    assert_eq!(summary["tool"]["bolt_holes"], 0);
    // none asked for: no tool
    let (commands, summary) = build(&json!({}), &names(&["A"]), &m).unwrap();
    assert!(!commands.iter().any(|c| c.1["name"] == "Worktop Tool shelf"));
    assert!(summary.get("tool").is_none());
}

#[test]
fn bolt_holes_for_the_tools_feet_go_through_its_shelf() {
    // the tool's middle at (600, -280): four 8 mm holes 400 apart across, 300 front to back
    let m = measure(vec![base("A", 0.0, 1000.0, 720.0)]);
    let p = tool(json!({"bolt_x": 400, "bolt_y": 300}));
    let (commands, summary) = build(&p, &names(&["A"]), &m).unwrap();
    let holes: Vec<&Value> = (commands.iter())
        .filter(|c| {
            c.1["name"]
                .as_str()
                .is_some_and(|n| n.starts_with("Worktop Tool bolt hole"))
        })
        .map(|c| &c.1)
        .collect();
    let bases: Vec<&Value> = holes.iter().map(|h| &h["base"]).collect();
    assert_eq!(
        bases,
        [
            &json!(["400 mm", "-430 mm", "440 mm"]),
            &json!(["400 mm", "-130 mm", "440 mm"]),
            &json!(["800 mm", "-430 mm", "440 mm"]),
            &json!(["800 mm", "-130 mm", "440 mm"])
        ]
    );
    assert!(holes
        .iter()
        .all(|h| h["targets"] == json!(["Worktop Tool shelf"])
            && h["diameter"] == "8 mm"
            && h["height"] == "18 mm"
            && h["operation"] == "cut"));
    assert_eq!(summary["tool"]["bolt_holes"], 4);
    // one spacing 0: a pair; holes off the shelf (it is 964 wide) are refused
    let pair = build(&tool(json!({"bolt_x": 400})), &names(&["A"]), &m).unwrap();
    assert_eq!(pair.1["tool"]["bolt_holes"], 2);
    let off = build(&tool(json!({"bolt_x": 1200})), &names(&["A"]), &m).unwrap_err();
    assert!(off.contains("off its shelf"), "{off}");
}

#[test]
fn the_tool_shelf_stops_at_a_grooved_back_or_the_wall() {
    // a back set in between the sides, 18 from the wall; a tool shallow enough to clear it
    let mut a = base("A", 0.0, 1000.0, 720.0);
    a[2]["bbox"] = json!({"min": [18.0, -24.0, 0.0], "max": [982.0, -18.0, 720.0]});
    let p = tool(json!({"tool_depth": 500}));
    let (commands, _) = build(&p, &names(&["A"]), &measure(vec![a])).unwrap();
    assert_eq!(named(&commands, "Worktop Tool shelf")["width"], "536 mm");
    // no back: to the sides' back edge
    let mut a = base("A", 0.0, 1000.0, 720.0);
    a.remove(2);
    let (commands, _) = build(&tool(json!({})), &names(&["A"]), &measure(vec![a])).unwrap();
    assert_eq!(named(&commands, "Worktop Tool shelf")["width"], "560 mm");
}

#[test]
fn what_a_tool_refuses() {
    let m = measure(vec![base("A", 0.0, 1000.0, 720.0)]);
    let why = refused(tool(json!({"tool_height": 20})), &["A"], &m);
    assert!(why.contains("above the worktop's underside"), "{why}");
    // as thick as the worktop is still fine: the shelf right under it
    assert!(build(&tool(json!({"tool_height": 38})), &names(&["A"]), &m).is_ok());
    // 758 - 750 - 18 - 40 is under the floor
    let why = refused(tool(json!({"tool_height": 750})), &["A"], &m);
    assert!(why.contains("below the cabinet's bottom"), "{why}");
    assert!(build(&tool(json!({"tool_height": 700})), &names(&["A"]), &m).is_ok());
    // with a bottom board whose top is at 100, the floor is there
    let mut a = base("A", 0.0, 1000.0, 720.0);
    a.push(body("A Bottom", [18.0, -560.0, 82.0], [982.0, 0.0, 100.0]));
    let why = refused(tool(json!({"tool_height": 650})), &["A"], &measure(vec![a]));
    assert!(why.contains("below the cabinet's bottom"), "{why}");
    // off the piece
    let why = refused(tool(json!({"tool_x": 100})), &["A"], &m);
    assert!(why.contains("the tool's opening does not fit"), "{why}");
    let why = refused(tool(json!({"tool_width": 0})), &["A"], &m);
    assert!(why.contains("more than 0"), "{why}");
    // across the joint of two cabinets
    let two = measure(vec![
        base("A", 0.0, 600.0, 720.0),
        base("B", 600.0, 600.0, 720.0),
    ]);
    let between = "the tool's opening must lie between the sides of one cabinet";
    assert_eq!(refused(tool(json!({})), &["A", "B"], &two), between);
    // in B alone it is fine
    let p = tool(json!({"tool_x": 900, "tool_width": 400}));
    let (_, summary) = build(&p, &names(&["A", "B"]), &two).unwrap();
    assert_eq!(summary["tool"]["cabinet"], "B");
    // in the front overhang, not over the carcass
    let p = tool(json!({"tool_depth": 20, "tool_y": 15}));
    assert_eq!(refused(p, &["A"], &m), between);
    // a corner cabinet's return side does not count as a side across the run
    let p = tool(json!({"tool_x": 750, "tool_width": 200, "tool_depth": 200}));
    let k = measure(vec![corner("K", 1000.0, 560.0)]);
    assert_eq!(refused(p, &["K"], &k), between);
}

#[test]
fn the_cabinets_top_and_rails_get_the_opening_and_a_shelf_is_in_the_way() {
    let mut a = base("A", 0.0, 1000.0, 720.0);
    a.push(body("A Top", [18.0, -560.0, 702.0], [982.0, 0.0, 720.0]));
    a.push(body(
        "A Rail front",
        [18.0, -560.0, 702.0],
        [982.0, -460.0, 720.0],
    ));
    a.push(body("A Bottom", [18.0, -560.0, 0.0], [982.0, 0.0, 18.0]));
    // a shelf under the tool's shelf is not in its way
    a.push(body(
        "A Shelf 2",
        [18.0, -560.0, 200.0],
        [982.0, -6.0, 218.0],
    ));
    let m = measure(vec![a.clone()]);
    let (_, summary) = build(&tool(json!({})), &names(&["A"]), &m).unwrap();
    assert_eq!(
        summary["tool"]["cut"]["targets"],
        json!(["A Top", "A Rail front"])
    );
    // a shelf between the tool's shelf and the worktop is
    a.push(body(
        "A Shelf 1",
        [18.0, -560.0, 500.0],
        [982.0, -6.0, 518.0],
    ));
    let why = refused(tool(json!({})), &["A"], &measure(vec![a]));
    assert_eq!(
        why,
        "`A Shelf 1` is in the tool's way: make the cabinet without it"
    );
    // the door in front, if the opening reaches over it
    let m = measure(vec![base("A", 0.0, 1000.0, 720.0)]);
    let p = tool(json!({"tool_y": 290, "tool_depth": 570}));
    assert!(refused(p, &["A"], &m).contains("`A Door` is in the tool's way"));
}

#[test]
fn a_world_box_in_a_cabinets_own_frame() {
    let world = [[100.0, -500.0, 700.0], [300.0, -100.0, 720.0]];
    // on the origin, square: as it is
    let square = json!([[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0]]);
    assert_eq!(
        local(world, &square),
        ([100.0, -500.0, 700.0], [200.0, 400.0, 20.0])
    );
    // moved 1000 along X and turned a quarter (its X along the world's Y)
    let turned = json!([[0, -1, 0, 1000], [1, 0, 0, 0], [0, 0, 1, 0]]);
    assert_eq!(
        local(world, &turned),
        ([-500.0, 700.0, 700.0], [400.0, 200.0, 20.0])
    );
}

#[test]
fn nothing_of_the_cabinet_where_the_shelf_and_battens_go() {
    let a = base("A", 0.0, 1000.0, 720.0);
    // a shelf of the cabinet at batten height (the tool's shelf from 440 to 458, battens 400 up)
    let mut shelf = a.clone();
    shelf.push(body(
        "A Shelf 1",
        [18.0, -560.0, 410.0],
        [982.0, -6.0, 428.0],
    ));
    let why = refused(tool(json!({})), &["A"], &measure(vec![shelf]));
    assert_eq!(
        why,
        "`A Shelf 1` is in the tool's way: make the cabinet without it"
    );
    // a divider beside the opening, in the shelf's way
    let mut divider = a.clone();
    divider.push(body(
        "A Divider 1",
        [100.0, -560.0, 18.0],
        [118.0, -6.0, 702.0],
    ));
    let why = refused(tool(json!({})), &["A"], &measure(vec![divider]));
    assert_eq!(
        why,
        "`A Divider 1` is in the tool's way: make the cabinet without it"
    );
    // just under the battens is fine
    let mut low = a;
    low.push(body(
        "A Shelf 1",
        [18.0, -560.0, 382.0],
        [982.0, -6.0, 400.0],
    ));
    assert!(build(&tool(json!({})), &names(&["A"]), &measure(vec![low])).is_ok());
}

#[test]
fn the_tools_opening_must_stop_at_the_cabinets_back() {
    let m = measure(vec![base("A", 0.0, 1000.0, 720.0)]);
    // the back's front face is 594 from the worktop's front edge: the opening to 594 is fine
    assert!(build(&tool(json!({"tool_y": 321})), &names(&["A"]), &m).is_ok());
    let why = refused(tool(json!({"tool_y": 322})), &["A"], &m);
    assert_eq!(why, "the tool's opening reaches over the cabinet's back");
}

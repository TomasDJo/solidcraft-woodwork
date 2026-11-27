use super::*;

/// Each body the commands make, with its corner and size in mm.
fn boxes(commands: &Commands) -> Vec<(String, [f64; 3], [f64; 3])> {
    let n = |v: &Value| {
        let s = v.as_str().unwrap_or("");
        let s = s.split(" - ").next().unwrap_or(s); // `at`'s "x - 0 mm/2"
        s.trim_end_matches(" mm")
            .trim()
            .parse::<f64>()
            .unwrap_or(f64::NAN)
    };
    (commands.iter())
        .filter(|c| c.0 == "solid.box" && c.1.get("operation").is_none())
        .map(|c| {
            let a = &c.1;
            let corner = [n(&a["corner"][0]), n(&a["corner"][1]), n(&a["corner"][2])];
            (
                a["name"].as_str().unwrap().to_string(),
                corner,
                [n(&a["length"]), n(&a["width"]), n(&a["height"])],
            )
        })
        .collect()
}

#[test]
fn a_frame_round_two_cabinets_with_a_cross_board_and_casters() {
    // two 600 x 560 cabinets side by side from (0, 0): frame 1200 x 560
    let plans = [[0.0, 0.0, 600.0, 560.0], [600.0, 0.0, 1200.0, 560.0]];
    let (commands, summary) = build(&json!({"name": "Bench"}), &plans).unwrap();
    let all = boxes(&commands);
    let find = |name: &str| all.iter().find(|b| b.0 == name).cloned().unwrap();
    assert_eq!(find("Bench Frame front").1, [0.0, 0.0, 100.0]);
    assert_eq!(find("Bench Frame front").2, [1200.0, 18.0, 70.0]);
    assert_eq!(find("Bench Frame back").1, [0.0, 542.0, 100.0]);
    assert_eq!(find("Bench Frame right").1, [1182.0, 18.0, 100.0]);
    // the joint at x = 600, centred
    assert_eq!(find("Bench Frame cross 1").1, [591.0, 18.0, 100.0]);
    assert!(all.iter().all(|b| b.0 != "Bench Frame cross 2"));
    // four corner blocks and casters: not long enough for a middle pair
    assert_eq!(summary["casters"], 4);
    assert_eq!(find("Bench Block 4").1, [1092.0, 452.0, 100.0]);
    assert_eq!(summary["lift"], 170.0);
    let casters = commands
        .iter()
        .filter(|c| {
            c.1["body_name"]
                .as_str()
                .is_some_and(|n| n.contains(" Caster "))
        })
        .count();
    assert_eq!(casters, 4);
}

#[test]
fn a_long_frame_gets_casters_in_its_middle_and_runs_either_way() {
    // three cabinets 600 wide: 1800 long, so a middle pair beside the cross board at 600 or 1200
    let plans = [
        [0.0, 0.0, 600.0, 500.0],
        [600.0, 0.0, 1200.0, 500.0],
        [1200.0, 0.0, 1800.0, 500.0],
    ];
    let (commands, summary) = build(&json!({"name": "B"}), &plans).unwrap();
    assert_eq!(summary["casters"], 6);
    let all = boxes(&commands);
    assert_eq!(
        all.iter()
            .filter(|b| b.0.starts_with("B Frame cross"))
            .count(),
        2
    );
    // a run along Y: the same frame turned
    let turned = [[0.0, 0.0, 500.0, 600.0], [0.0, 600.0, 500.0, 1200.0]];
    let all = boxes(&build(&json!({"name": "B"}), &turned).unwrap().0);
    let cross = all.iter().find(|b| b.0 == "B Frame cross 1").unwrap();
    assert_eq!(
        (cross.1, cross.2),
        ([18.0, 591.0, 100.0], [464.0, 18.0, 70.0])
    );
    // two narrow cabinets side by side, the frame deeper than wide: the joint at x = 250 still
    // gets its cross board, along the frame's long way
    let narrow = [[0.0, 0.0, 250.0, 560.0], [250.0, 0.0, 500.0, 560.0]];
    let all = boxes(&build(&json!({"name": "B"}), &narrow).unwrap().0);
    let cross = all.iter().find(|b| b.0 == "B Frame cross 1").unwrap();
    assert_eq!(
        (cross.1, cross.2),
        ([241.0, 18.0, 100.0], [18.0, 524.0, 70.0])
    );
}

#[test]
fn what_a_base_refuses() {
    let tiny = [[0.0, 0.0, 200.0, 200.0]];
    assert!(build(&json!({}), &tiny).unwrap_err().contains("too small"));
    let ok = [[0.0, 0.0, 600.0, 560.0]];
    assert!(build(&json!({"caster_height": 30}), &ok)
        .unwrap_err()
        .contains("caster height"));
    assert!(build(&json!({"frame_height": 10}), &ok)
        .unwrap_err()
        .contains("frame height"));
}

#[test]
fn the_plans_and_floor_come_from_the_sides_and_backs() {
    let body = |name: &str, min: [f64; 3], max: [f64; 3]| json!({"name": name, "bbox": {"min": min, "max": max}});
    let measure = json!({"bodies": [
        body("A Side L", [0.0, 0.0, 0.0], [18.0, 560.0, 720.0]),
        body("A Side R", [582.0, 0.0, 0.0], [600.0, 560.0, 720.0]),
        body("A Door", [0.0, -20.0, 2.0], [600.0, 0.0, 718.0]), // in front: not the footprint
        body("A Leg 1", [30.0, 30.0, -100.0], [70.0, 70.0, 0.0]),
    ]});
    let (plans, floor) = plans(&measure, &["A".to_string()]).unwrap();
    assert_eq!(plans, [[0.0, 0.0, 600.0, 560.0]]);
    assert_eq!(floor, [-100.0]); // each cabinet its own floor
    assert!(super::plans(&measure, &["B".to_string()]).is_err());
}

#[test]
fn each_cabinet_has_its_own_floor() {
    // A on the floor, B already 100 up (on an earlier base): each goes up by its own height
    let body = |name: &str, x: f64, z: f64| json!({"name": name, "bbox": {"min": [x, 0.0, z], "max": [x + 18.0, 560.0, z + 720.0]}});
    let measure = json!({"bodies": [
        body("A Side L", 0.0, 0.0), body("A Side R", 582.0, 0.0),
        body("B Side L", 600.0, 100.0), body("B Side R", 1182.0, 100.0),
    ]});
    let (_, floors) = plans(&measure, &["A".to_string(), "B".to_string()]).unwrap();
    assert_eq!(floors, [0.0, 100.0]);
}

#[test]
fn the_dialogs_bounds_are_the_builders() {
    // the manifest's Mobile base fields allow exactly what `build` takes (mm only, no Units)
    let manifest: Value = serde_json::from_str(include_str!("../../plugin.json")).unwrap();
    let entries = manifest["runtime"]["entries"].as_array().unwrap();
    let schema = &entries.iter().find(|e| e["name"] == "base").unwrap()["input_schema"];
    assert!(schema["properties"].get("units").is_none());
    let ok = [[0.0, 0.0, 600.0, 560.0]];
    for (key, low, high) in [
        ("caster_height", 50.0, 300.0),
        ("frame_height", 30.0, 300.0),
        ("thickness", 3.0, 50.0),
    ] {
        let field = &schema["properties"][key];
        assert_eq!(
            (field["minimum"].as_f64(), field["maximum"].as_f64()),
            (Some(low), Some(high)),
            "{key}"
        );
        let at = |v: f64| build(&json!({ key: v }), &ok);
        assert!(
            at(low).is_ok() && at(high).is_ok(),
            "{key}: its bounds are taken"
        );
        assert!(
            at(low - 0.5).is_err() && at(high + 0.5).is_err(),
            "{key}: past them refused"
        );
    }
}

#[test]
fn connector_bolts_go_through_the_touching_sides() {
    // A from x 0 to 600, B beside it to 1200, C apart at 2000: four holes through A Side R and
    // B Side L, none to C
    let side = |name: &str, x0: f64| json!({"name": name, "bbox": {"min": [x0, 0, 170], "max": [x0 + 18.0, 560, 890]}});
    let m = json!({"bodies": [side("A Side L", 0.0), side("A Side R", 582.0), side("B Side L", 600.0),
        side("B Side R", 1182.0), side("C Side L", 2000.0), side("C Side R", 2582.0)]});
    let names = ["A", "B", "C"].map(String::from);
    let holes = bolts(&m, &names);
    assert_eq!(holes.len(), 4);
    assert!(holes
        .iter()
        .all(|h| h.0 == ["A Side R".to_string(), "B Side L".to_string()]));
    // through both sides, at a quarter of the depth and of the height
    assert_eq!(holes[0].1, [[582.0, 135.5, 345.5], [618.0, 144.5, 354.5]]);
    assert_eq!(holes[3].1[0], [582.0, 415.5, 705.5]);
    // a run along Y is found the same way
    let turned = |name: &str, y0: f64| json!({"name": name, "bbox": {"min": [0, y0, 0], "max": [560, y0 + 18.0, 720]}});
    let m = json!({"bodies": [turned("A Side R", 582.0), turned("B Side L", 600.0)]});
    assert_eq!(bolts(&m, &names[..2]).len(), 4);
}

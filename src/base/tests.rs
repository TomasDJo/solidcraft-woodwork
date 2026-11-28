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

/// The frame boards and blocks among the boxes that share any volume.
fn overlapping(all: &[(String, [f64; 3], [f64; 3])]) -> Vec<(String, String)> {
    let mut found = Vec::new();
    let all: Vec<_> = (all.iter())
        .filter(|b| b.0.contains(" Frame ") || b.0.contains(" Block "))
        .collect();
    let all = &all[..];
    for (i, a) in all.iter().enumerate() {
        for b in &all[i + 1..] {
            if (0..3).all(|k| a.1[k] < b.1[k] + b.2[k] - 0.01 && b.1[k] < a.1[k] + a.2[k] - 0.01) {
                found.push((a.0.clone(), b.0.clone()));
            }
        }
    }
    found
}

#[test]
fn cabinets_back_to_back_of_different_widths_get_boards_that_do_not_cross() {
    // #216: A 600 x 560 and B 800 x 560 behind it, back to back, both from x = 0. The joint at
    // y = 560 gets a board across; A's right side at x = 600 one along, only under A and short of
    // the board across, so nothing overlaps.
    let plans = [[0.0, 0.0, 600.0, 560.0], [0.0, 560.0, 800.0, 1120.0]];
    let (commands, _) = build(&json!({"name": "B"}), &plans).unwrap();
    let all = boxes(&commands);
    assert_eq!(overlapping(&all), vec![] as Vec<(String, String)>);
    let find = |name: &str| all.iter().find(|b| b.0 == name).cloned().unwrap();
    assert_eq!(find("B Frame cross 1").1, [18.0, 551.0, 100.0]);
    assert_eq!(find("B Frame cross 2").1, [591.0, 18.0, 100.0]);
    assert_eq!(find("B Frame cross 2").2, [18.0, 533.0, 70.0]);
    assert!(all.iter().all(|b| b.0 != "B Frame cross 3"));
    // offset the other way, B's edge at x = 200 inside A's span: under B only
    let plans = [[0.0, 0.0, 800.0, 560.0], [200.0, 560.0, 800.0, 1120.0]];
    let all = boxes(&build(&json!({"name": "B"}), &plans).unwrap().0);
    assert_eq!(overlapping(&all), vec![] as Vec<(String, String)>);
    let cross = all.iter().find(|b| b.0 == "B Frame cross 2").unwrap();
    assert_eq!((cross.1[1], cross.2[1]), (569.0, 533.0));
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
fn casters_go_in_the_corners_and_evenly_between() {
    // #228: at most 1500 apart, evenly spaced, not at the cross boards
    assert_eq!(casters(0.0, 1400.0, &[600.0], 18.0).unwrap(), [0.0, 1400.0]);
    let three = casters(0.0, 3200.0, &[], 18.0).unwrap();
    assert_eq!(three.len(), 4);
    assert!((three[1] - 3200.0 / 3.0).abs() < 1e-9 && (three[2] - 6400.0 / 3.0).abs() < 1e-9);
    // a block that would stand on a board across moves to its nearer side
    assert_eq!(
        casters(0.0, 1600.0, &[800.0], 18.0).unwrap(),
        [0.0, 809.0, 1600.0]
    );
    assert_eq!(
        casters(0.0, 1600.0, &[850.0], 18.0).unwrap(),
        [0.0, 751.0, 1600.0]
    );
    // clear of every board, not only the first it meets: boards at 1100 and 1150
    let near = casters(18.0, 3192.0, &[1100.0, 1150.0, 2200.0], 18.0).unwrap();
    let on_board = |a: &f64| {
        [1100.0, 1150.0, 2200.0]
            .iter()
            .any(|c| *a < c + 9.0 && a + 90.0 > c - 9.0)
    };
    assert!(!near.iter().any(on_board), "{near:?}");
    // a move that would leave casters more than 1500 apart brings a pair more instead
    for (last, board) in [(3018.0, 1563.0), (3018.0, 1518.0), (3100.0, 1550.0)] {
        let at = casters(18.0, last, &[board], 18.0).unwrap();
        assert!(at.windows(2).all(|w| w[1] - w[0] <= LONG), "{at:?}");
        assert!(
            !at.iter()
                .any(|a| *a < board + 9.0 && a + 90.0 > board - 9.0),
            "{at:?}"
        );
    }
    // boards across every 50 mm the whole way: no room for a block, refused
    let packed: Vec<f64> = (1..60).map(|k| 18.0 + k as f64 * 50.0).collect();
    assert!(casters(18.0, 3018.0, &packed, 18.0)
        .unwrap_err()
        .contains("no room"));
    // the review's case: 150 mm boards every 200 mm from 1000 to 3000 leave a 2240 mm stretch
    // with no place for a block, refused rather than a gap over 1500
    let wide: Vec<f64> = (0..=10).map(|k| 1000.0 + k as f64 * 200.0).collect();
    assert!(casters(150.0, 4160.0, &wide, 150.0).is_err());
}

#[test]
fn stiffeners_keep_the_boards_across_within_the_spacing() {
    // #229: ends at 0 and 2400, a cross board at 1000: 1000 and 1400 apart, so one stiffener in
    // the first stretch and two in the second
    assert_eq!(
        stiffeners(0.0, 2400.0, &[1000.0], 600.0),
        [500.0, 1000.0 + 1400.0 / 3.0, 1000.0 + 2800.0 / 3.0]
    );
    assert_eq!(stiffeners(0.0, 600.0, &[], 600.0), [] as [f64; 0]);
    // in a frame: a 2400 mm cabinet gets its stiffeners, nothing overlaps, and the spacing is checked
    let plans = [[0.0, 0.0, 2400.0, 560.0]];
    let all = boxes(&build(&json!({"name": "S"}), &plans).unwrap().0);
    assert_eq!(
        all.iter()
            .filter(|b| b.0.starts_with("S Frame stiffener"))
            .count(),
        3
    );
    assert_eq!(overlapping(&all), vec![] as Vec<(String, String)>);
    let wide = build(&json!({"name": "S", "stiffener_spacing": 1000}), &plans).unwrap();
    assert_eq!(
        boxes(&wide.0)
            .iter()
            .filter(|b| b.0.contains("stiffener"))
            .count(),
        2
    );
    assert!(build(&json!({"stiffener_spacing": 100}), &plans).is_err());
    // 150 mm timber at 300 mm spacing on a 600 frame: the stiffener would stand on the corner
    // blocks, so it is refused with a word on the spacing
    let small = [[0.0, 0.0, 600.0, 600.0]];
    let thick =
        json!({"frame_material": "timber", "timber_thickness": 150, "stiffener_spacing": 300});
    assert!(build(&thick, &small)
        .unwrap_err()
        .contains("stiffener spacing"));
}

#[test]
fn a_picked_board_names_its_base_and_fills_the_dialog() {
    // #239
    let files = ["base/Bench.json", "base/Bench_2.json", "edit/Bench.json"];
    assert_eq!(owner("Bench Frame front", &files), Some("Bench"));
    assert_eq!(owner("Bench 2 Block 3", &files), Some("Bench_2"));
    assert_eq!(owner("Bench 2 Caster 1", &files), Some("Bench_2"));
    assert_eq!(owner("Bench Side L", &files), None);
    assert_eq!(owner("Other Frame front", &files), None);
    // a cabinet called "Bench Frame" is no part of base Bench
    assert_eq!(owner("Bench Frame Side L", &files), None);
    assert_eq!(owner("Bench Frame stiffener 2", &files), Some("Bench"));
    assert_eq!(owner("Bench Frame cross 1", &files), Some("Bench"));
    assert_eq!(file("Bench 2"), "base/Bench_2.json");
    let kept = json!({"name": "Bench", "frame_height": 100, "selection": ["x"], "_extra": 1});
    let reply = fields(&kept);
    assert_eq!(reply["frame_height"], json!({"default": 100}));
    assert_eq!(reply["name"]["default"], "Bench");
    assert!(reply["name"]["hint"]
        .as_str()
        .unwrap()
        .starts_with("Editing Bench"));
    assert!(reply.get("selection").is_none() && reply.get("_extra").is_none());
}

#[test]
fn a_new_footprint_needs_the_frame_again() {
    // #198
    let changed = |fields: &[&str]| fields.iter().map(|f| f.to_string()).collect::<Vec<_>>();
    assert!(resized(&changed(&["shelves", "width"])));
    assert!(resized(&changed(&["return"])));
    assert!(!resized(&changed(&["height", "doors", "handles"])));
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
        ("stiffener_spacing", 300.0, 2000.0),
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
    // a timber section's two sizes
    for (key, low, high) in [
        ("timber_thickness", 20.0, 150.0),
        ("timber_width", 30.0, 300.0),
    ] {
        let field = &schema["properties"][key];
        assert_eq!(
            (field["minimum"].as_f64(), field["maximum"].as_f64()),
            (Some(low), Some(high)),
            "{key}"
        );
        let at = |v: f64| build(&json!({"frame_material": "timber", key: v}), &ok);
        assert!(at(low).is_ok() && at(high).is_ok(), "{key}");
        assert!(at(low - 0.5).is_err() && at(high + 0.5).is_err(), "{key}");
    }
}

#[test]
fn a_caster_block_is_told_by_its_name() {
    assert!(is_block("Cart Block 3"));
    assert!(!is_block("Cart Block x"));
    assert!(!is_block("Cart Frame front"));
}

#[test]
fn a_timber_frame_takes_its_sizes_from_the_section() {
    // 48 x 98 timber by default: the frame 48 across and 98 tall, whatever Frame height and
    // Thickness say; a US 2x4 is 38 x 89
    let plans = [[0.0, 0.0, 600.0, 560.0]];
    let p = json!({"name": "T", "frame_material": "timber", "frame_height": 70, "thickness": 18});
    let all = boxes(&build(&p, &plans).unwrap().0);
    let front = all.iter().find(|b| b.0 == "T Frame front").unwrap();
    assert_eq!(front.2, [600.0, 48.0, 98.0]);
    let us = json!({"frame_material": "timber", "timber_thickness": 38, "timber_width": 89});
    assert_eq!(timber(&us).unwrap(), Some((38.0, 89.0)));
    assert_eq!(timber(&json!({"timber_width": 89})).unwrap(), None);
}

#[test]
fn connector_bolts_go_through_the_touching_sides() {
    // A from x 0 to 600, B beside it to 1200, C apart at 2000: four holes through A Side R and
    // B Side L, none to C
    let side = |name: &str, x0: f64| json!({"name": name, "bbox": {"min": [x0, 0, 170], "max": [x0 + 18.0, 560, 890]}});
    let m = json!({"bodies": [side("A Side L", 0.0), side("A Side R", 582.0), side("B Side L", 600.0),
        side("B Side R", 1182.0), side("C Side L", 2000.0), side("C Side R", 2582.0)]});
    let names = ["A", "B", "C"].map(String::from);
    let holes = bolts(&m, &names).unwrap();
    assert_eq!(holes.len(), 4);
    assert!(holes
        .iter()
        .all(|h| h.0 == ["A Side R".to_string(), "B Side L".to_string()]));
    assert_eq!(holes.iter().map(|h| h.1).collect::<Vec<_>>(), [1, 2, 3, 4]);
    // through both sides, at a quarter of the depth and of the height
    assert_eq!(holes[0].2, [[582.0, 135.5, 345.5], [618.0, 144.5, 354.5]]);
    assert_eq!(holes[3].2[0], [582.0, 415.5, 705.5]);
    // a counterbore (#260) 20 across and 6 deep on each side's outer face: A Side R's at x 582,
    // B Side L's at 618
    let [a, b] = holes[0].3;
    assert_eq!(a, [[582.0, 130.0, 340.0], [588.0, 150.0, 360.0]]);
    assert_eq!((b[0][0], b[1][0]), (612.0, 618.0));
    // sides too narrow for a counterbore a quarter in get no bolts
    let low = |name: &str, x0: f64| json!({"name": name, "bbox": {"min": [x0, 0, 0], "max": [x0 + 18.0, 560, 79]}});
    let m2 = json!({"bodies": [low("A Side R", 582.0), low("B Side L", 600.0)]});
    assert!(bolts(&m2, &names[..2]).unwrap().is_empty());
    // sides too thin to leave wood under a counterbore are refused, not cut through
    let thin = |name: &str, x0: f64| json!({"name": name, "bbox": {"min": [x0, 0, 170], "max": [x0 + 6.0, 560, 890]}});
    let m3 = json!({"bodies": [thin("A Side R", 594.0), thin("B Side L", 600.0)]});
    let err = bolts(&m3, &names[..2]).unwrap_err();
    assert!(err.contains("too thin") && err.contains("12 mm"), "{err}");
    // picked in another order, or only some of them: the same pairs, numbers and holes
    let again = bolts(&m, &["C", "B", "A"].map(String::from)).unwrap();
    assert_eq!(again, holes);
    assert_eq!(bolts(&m, &["B", "A"].map(String::from)).unwrap(), holes);
    // a run along Y is found the same way
    let turned = |name: &str, y0: f64| json!({"name": name, "bbox": {"min": [0, y0, 0], "max": [560, y0 + 18.0, 720]}});
    let m = json!({"bodies": [turned("A Side R", 582.0), turned("B Side L", 600.0)]});
    assert_eq!(bolts(&m, &names[..2]).unwrap().len(), 4);
}

#[test]
fn screws_go_down_through_the_bottoms_into_the_frame() {
    // A's bottom from x 18 to 582, y 0 to 560, on the frame's front and back boards (18 thick,
    // top at 170): two holes each, on their middle lines; the left board under Side L, none
    let body = |name: &str, min: [f64; 3], max: [f64; 3]| json!({"name": name, "bbox": {"min": min, "max": max}});
    let m = json!({"bodies": [
        body("A Bottom", [18.0, 0.0, 170.0], [582.0, 560.0, 188.0]),
        body("Bench Frame front", [0.0, 0.0, 100.0], [600.0, 18.0, 170.0]),
        body("Bench Frame back", [0.0, 542.0, 100.0], [600.0, 560.0, 170.0]),
        body("Bench Frame left", [0.0, 18.0, 100.0], [18.0, 542.0, 170.0]),
        body("Other Frame front", [0.0, 0.0, 100.0], [600.0, 18.0, 170.0]),
    ]});
    let holes = screws(&m, &["A".to_string()], "Bench");
    assert_eq!(holes.len(), 4);
    assert_eq!(
        (holes[0].0.as_str(), holes[0].1.as_str(), holes[0].2),
        ("A Bottom", "Bench Frame front", 1)
    );
    assert_eq!(holes[0].3, [[156.75, 6.75, 170.0], [161.25, 11.25, 188.0]]);
    assert_eq!(holes[3].1, "Bench Frame back");
    // a split corner's return bottom is screwed down too
    let mut split = m.clone();
    split["bodies"][0]["name"] = json!("A Bottom return");
    let holes = screws(&split, &["A".to_string()], "Bench");
    assert_eq!(holes.len(), 4);
    assert_eq!(holes[0].0, "A Bottom return");
    // a bottom that does not rest on the frame (a plinth under it) gets none
    let mut high = m.clone();
    high["bodies"][0]["bbox"]["min"][2] = json!(270.0);
    assert!(screws(&high, &["A".to_string()], "Bench").is_empty());
}

#[test]
fn a_frame_made_again_replaces_moved_screws_and_keeps_the_rest() {
    let hole = |y: f64| [[156.75, y - 2.25, 170.0], [161.25, y + 2.25, 188.0]];
    let screw =
        |n: usize, board: &str, y: f64| ("A Bottom".to_string(), board.to_string(), n, hole(y));
    // the front board grew from 18 to 40 mm: its middle line moved from y 9 to y 20
    let old = screw_name(&screw(1, "Bench Frame front", 9.0));
    let new = screw_name(&screw(1, "Bench Frame front", 20.0));
    assert_eq!(
        old,
        "A Bottom frame screw 1 into Bench Frame front at 159, 9"
    );
    assert_ne!(old, new);
    let back = screw_name(&screw(1, "Bench Frame back", 551.0));
    let had = [
        old.as_str(),
        back.as_str(),
        "B Bottom frame screw 1 into Bench Frame front at 759, 9",
        "A Bottom return frame screw 2 into Bench Frame left at 9, 300",
        "A Side L pins",
    ];
    // the moved one and the return's that is no longer wanted go; the unmoved back one, B's
    // and anything else stay
    let wanted = [new.clone(), back.clone()];
    assert_eq!(
        stale_screws(&had, &wanted, "A"),
        [
            old.as_str(),
            "A Bottom return frame screw 2 into Bench Frame left at 9, 300"
        ]
    );
    // an identical run again deletes nothing
    assert!(stale_screws(&[back.as_str()], std::slice::from_ref(&back), "A").is_empty());
}

use super::*;

fn body(name: &str, min: [f64; 3], max: [f64; 3]) -> Value {
    json!({"name": name, "bbox": {"min": min, "max": max}, "material": "Oak",
           "volume_mm3": (0..3).map(|i| max[i] - min[i]).product::<f64>()})
}

/// `Base`, 600 x 720 x 560 on the origin with a grooved back and a door, and `Base 2` beside
/// it, its sides only.
fn measure() -> Value {
    json!({"bodies": [
        body("Base Side L", [-300.0, -280.0, 0.0], [-282.0, 280.0, 720.0]),
        body("Base Side R", [282.0, -280.0, 0.0], [300.0, 280.0, 720.0]),
        body("Base Back", [-291.0, 262.0, 9.0], [291.0, 268.0, 711.0]),
        body("Base Door", [-298.0, -300.0, 2.0], [298.0, -282.0, 718.0]),
        body("Base 2 Side L", [300.0, -280.0, 0.0], [318.0, 280.0, 720.0]),
        body("Base 2 Side R", [882.0, -280.0, 0.0], [900.0, 280.0, 720.0]),
        body("Worktop", [-300.0, -300.0, 720.0], [900.0, 280.0, 750.0]),
    ]})
}

/// Base in a Kitchen component: occurrence 2 of the Kitchen, 4 of Base.
fn tree() -> Value {
    json!({"components": [
        {"id": 0, "name": "kitchen", "occurrences": []},
        {"id": 1, "name": "Kitchen", "occurrences": [{"id": 2, "parent": 0}]},
        {"id": 3, "name": "Base", "occurrences": [{"id": 4, "parent": 1}]},
        {"id": 5, "name": "Base Door", "occurrences": [{"id": 6, "parent": 3}]},
        {"id": 7, "name": "Base 2", "occurrences": [{"id": 8, "parent": 1}]},
    ]})
}

fn joints() -> Value {
    let names = [
        "Base Door hinge",
        "Base link",
        "Base 2 Door hinge",
        "Base 2 Drawer 1 slide",
    ];
    json!({"joints": names.map(|n| json!({"name": n}))})
}

#[test]
fn refuses_more_than_a_run_can_do() {
    assert!(budget(20, 8).is_ok()); // 8 + 240 + 8 = 256
    let err = budget(20, 9).unwrap_err();
    assert!(
        err.starts_with("20 cabinets need about 257 commands"),
        "{err}"
    );
    assert!(err.contains("Selected cabinet"), "{err}");
}

#[test]
fn finds_every_cabinet_and_where_it_stands() {
    assert_eq!(cabinets(&measure()), ["Base", "Base 2"]);
    assert_eq!(path(&tree(), "Base").unwrap(), (3, vec![2, 4]));
    assert_eq!(path(&tree(), "Base Door").unwrap(), (5, vec![2, 4, 6]));
    assert_eq!(path(&tree(), "Tall").unwrap_err(), "no component `Tall`");
}

#[test]
fn closes_the_cabinets_own_doors_and_drawers() {
    let m = measure();
    assert_eq!(
        fronts(&joints(), &m, "Base"),
        [json!({"joint": "Base Door hinge", "values": [0]})]
    );
    assert_eq!(fronts(&joints(), &m, "Base 2").len(), 2);
    assert!(fronts(&json!({}), &m, "Base").is_empty());
}

#[test]
fn reads_boxes_and_how_far_they_reach() {
    let m = measure();
    let side = bbox(&m, "Base Side R").unwrap();
    assert_eq!(side, [[282.0, -280.0, 0.0], [300.0, 280.0, 720.0]]);
    assert!(bbox(&m, "Base Shelf").is_none());
    assert_eq!(reach(&side, [1.0, 0.0]), 300.0);
    assert_eq!(reach(&side, [-1.0, 0.0]), -282.0);
    assert_eq!(reach(&side, [0.0, -1.0]), 280.0);
}

#[test]
fn tells_which_way_a_cabinet_faces() {
    let m = measure();
    let (l, r) = (
        bbox(&m, "Base Side L").unwrap(),
        bbox(&m, "Base Side R").unwrap(),
    );
    assert_eq!(facing(&l, &r), Some(([1.0, 0.0], [0.0, -1.0])));
    // turned a quarter clockwise: Side R towards -Y, the front towards -X
    let turned = |b: Box3| [[b[0][1], -b[1][0], 0.0], [b[1][1], -b[0][0], 720.0]];
    assert_eq!(
        facing(&turned(l), &turned(r)),
        Some(([0.0, -1.0], [-1.0, 0.0]))
    );
    // a corner cabinet's sides stand square to each other
    assert_eq!(facing(&l, &turned(r)), None);
    assert_eq!(views([0.0, -1.0]), ("front", "iso"));
    assert_eq!(views([-1.0, 0.0]), ("left", "iso_front_left"));
    assert_eq!(views([1.0, 0.0]), ("right", "iso_back_right"));
    assert_eq!(views([0.0, 1.0]), ("back", "iso_back_left"));
}

#[test]
fn draws_at_the_largest_scale_that_fits() {
    let room = [165.0, 242.0];
    assert_eq!(fit([1180.0, 720.0], room), 8); // a base cabinet, 1:10 before
    assert_eq!(fit([1180.0, 2100.0], room), 10); // a tall one, 1:20 before
    assert_eq!(fit([1820.0, 720.0], room), 15); // a 900 x 900 corner, 1:20 before
    assert_eq!(fit([300.0, 150.0], room), 2);
    assert_eq!(fit([600.0, 300.0], room), 4);
    assert_eq!(fit([9000.0, 3000.0], room), 50);
    let [w, h] = iso_size(900.0, 720.0, 920.0);
    assert!(
        (w - 1286.9).abs() < 0.1 && (h - 1330.8).abs() < 0.1,
        "{w} {h}"
    );
}

#[test]
fn looks_into_a_corner_cabinets_open_corner() {
    // a left corner, as built: Side L ends the return at the front, Side R the back leg
    let l = [[-450.0, -450.0, 0.0], [110.0, -432.0, 720.0]];
    let r = [[432.0, -110.0, 0.0], [450.0, 450.0, 720.0]];
    assert_eq!(corner_views(&l, &r), ([0.0, -1.0], "iso"));
    // a right corner, mirrored: open at the front left, not drawn from the front right
    let mirror = |b: Box3| [[-b[1][0], b[0][1], 0.0], [-b[0][0], b[1][1], 720.0]];
    assert_eq!(
        corner_views(&mirror(r), &mirror(l)),
        ([0.0, -1.0], "iso_front_left")
    );
    // turned half round: open at the back
    let back = |b: Box3| [[-b[1][0], -b[1][1], 0.0], [-b[0][0], -b[0][1], 720.0]];
    assert_eq!(
        corner_views(&back(l), &back(r)),
        ([0.0, 1.0], "iso_back_left")
    );
}

#[test]
fn lists_the_boards_in_columns() {
    let m = measure();
    let own = json!({"bodies": m["bodies"].as_array().unwrap()[..4]});
    let notes = board_list(&own, &Local::new(), "Base").unwrap();
    let texts: Vec<&str> = notes.iter().map(|(t, _)| t.as_str()).collect();
    assert_eq!(texts[0], "Qty\n1\n1\n2");
    assert_eq!(texts[1], "Part\nBack\nDoor\nSide L, Side R");
    assert_eq!(
        texts[2],
        "L x W x T\n702 x 582 x 6\n716 x 596 x 18\n720 x 560 x 18"
    );
    assert_eq!(texts[3], "Material\nOak\nOak\nOak");
    // each column starts where the one before it ends, 5 mm on
    let width = |text: &str| crate::pdf::width(text, SIZE);
    assert_eq!(notes[0].1, LIST_LEFT + width("Qty") / 2.0);
    let part = LIST_LEFT + width("Qty") + 5.0 + width("Side L, Side R") / 2.0;
    assert!((notes[1].1 - part).abs() < 1e-9);
    // a long list of names goes on over lines, and what does not fit is cut off
    let many: Vec<Value> = (0..60)
        .map(|i| {
            body(
                &format!("Base Shelf {i}"),
                [0.0; 3],
                [100.0 + i as f64, 300.0, 18.0],
            )
        })
        .collect();
    let notes = board_list(&json!({ "bodies": many }), &Local::new(), "Base").unwrap();
    let lines: Vec<&str> = notes[1].0.lines().collect();
    assert_eq!(lines.len(), ((TOP + 2.0 - LIST_BOTTOM) / PITCH) as usize);
    assert_eq!(lines.last(), Some(&"... the rest: see the cut list"));
    // no blank lines at the end of a column
    assert_eq!(notes[0].0.lines().count(), lines.len() - 1);
    assert!(!notes[0].0.ends_with('\n'));
    let mut same = vec![body("Base Side L", [0.0; 3], [18.0, 560.0, 720.0])];
    same.extend(
        (0..12).map(|i| body(&format!("Base Divider {i}"), [0.0; 3], [18.0, 560.0, 720.0])),
    );
    let notes = board_list(&json!({ "bodies": same }), &Local::new(), "Base").unwrap();
    assert!(notes[1].0.lines().count() > 2);
    assert_eq!(notes[0].0, "Qty\n13");
}

#[test]
fn draws_a_sheet_per_cabinet() {
    let m = measure();
    let steps = sheet(&m, &Local::new(), &tree(), &joints(), "Base").unwrap();
    let commands: Vec<&str> = steps.iter().map(|s| s.0).collect();
    assert_eq!(
        commands,
        [
            "drawing.sheet.add",
            "drawing.view.base",
            "drawing.view.projected",
            "drawing.view.base",
            "drawing.dimension.add",
            "drawing.dimension.add",
            "drawing.dimension.add",
            "drawing.note.add",
            "drawing.note.add",
            "drawing.note.add",
            "drawing.note.add",
        ]
    );
    assert_eq!(steps.len() + 1, PER_SHEET); // and the old sheet's deletion
    let kept: Vec<&str> = steps
        .iter()
        .map(|s| s.2)
        .filter(|k| !k.is_empty())
        .collect();
    assert_eq!(kept, ["sheet", "front", "right"]);
    assert_eq!(
        steps[0].1,
        json!({"name": "Base", "documents": {"source": "$source", "component": 3}})
    );
    // 600 x 720 x 580 at 1:8: the front view 75 x 90, with the right view in the middle of
    // the room left of the board list
    let front = &steps[1].1;
    assert_eq!(front["view"], "front");
    assert_eq!(front["scale"], "1:8");
    assert_eq!(front["position_mm"], json!([86.25, 161.0]));
    assert_eq!(front["source"], json!({"occurrences": [[2, 4]]}));
    assert_eq!(
        front["pose"],
        json!([{"joint": "Base Door hinge", "values": [0]}])
    );
    assert_eq!(steps[2].1, json!({"parent": "$front", "side": "right"}));
    // the iso view under the board list, right of the others, at its own scale
    assert_eq!(steps[3].1["view"], "iso");
    assert_eq!(steps[3].1["scale"], "1:8");
    let at = &steps[3].1["position_mm"];
    assert_eq!(at[0], json!(320.0));
    assert!((at[1].as_f64().unwrap() - 159.9).abs() < 0.01, "{at}");
    // width and height on Side L and Side R, under and left of the front view; the depth from
    // Side R's front to its rear, the grooved back standing inside it
    let dim = |i: usize| &steps[i].1;
    let ends = |i: usize| {
        let linear = &dim(i)["kind"]["linear"];
        (
            linear["a"]["point"].clone(),
            linear["b"]["point"].clone(),
            linear["direction"].clone(),
        )
    };
    assert_eq!(
        ends(4),
        (
            json!([-300.0, -280.0, 0.0]),
            json!([300.0, -280.0, 0.0]),
            json!("horizontal")
        )
    );
    assert_eq!(
        ends(5),
        (
            json!([-300.0, -280.0, 0.0]),
            json!([-300.0, -280.0, 720.0]),
            json!("vertical")
        )
    );
    assert_eq!(
        ends(6),
        (
            json!([300.0, -280.0, 0.0]),
            json!([300.0, 280.0, 0.0]),
            json!("horizontal")
        )
    );
    assert_eq!(dim(4)["kind"]["linear"]["a"]["path"], json!([2, 4]));
    assert_eq!(
        (dim(4)["view"].clone(), dim(6)["view"].clone()),
        (json!("$front"), json!("$right"))
    );
    assert_eq!(dim(4)["line_through_mm"], json!([86.25, 106.0]));
    assert_eq!(dim(5)["line_through_mm"], json!([38.75, 161.0]));
    assert_eq!(dim(4)["decimals"], 0);
    let note = &steps[7].1;
    assert_eq!(note["anchor"], json!({"sheet": "$sheet"}));
    assert_eq!(note["cap_mm"], CAP);
    // its middle: the list's top less half of its four lines
    assert_eq!(note["at"][1], json!(TOP + 2.0 - 2.0 * PITCH));
    assert!(note["content"].as_str().unwrap().starts_with("Qty\n"));

    // an overlaid back reaches past the sides: the depth goes to its own corner
    let mut over = m.clone();
    over["bodies"][2] = body("Base Back", [-300.0, 280.0, 9.0], [300.0, 298.0, 711.0]);
    let steps = sheet(&over, &Local::new(), &tree(), &joints(), "Base").unwrap();
    assert_eq!(
        steps[6].1["kind"]["linear"]["b"]["point"],
        json!([300.0, 298.0, 9.0])
    );

    // a corner cabinet gets its views and boards, no dimensions; without joints no pose
    let mut corner = m.clone();
    corner["bodies"][1] = body("Base Side R", [-300.0, 262.0, 0.0], [300.0, 280.0, 720.0]);
    let steps = sheet(&corner, &Local::new(), &tree(), &json!({}), "Base").unwrap();
    assert_eq!(steps.len(), 8);
    assert!(steps[1].1.get("pose").is_none());
    assert!(steps.iter().all(|s| s.0 != "drawing.dimension.add"));
    let err = sheet(&m, &Local::new(), &tree(), &joints(), "Tall").unwrap_err();
    assert_eq!(err, "no component `Tall`");
}

#[test]
fn replaces_the_sheets_of_cabinets_drawn_again() {
    let drawing = json!({"sheets": [{"id": "a", "name": "Sheet 1"}, {"id": "b", "name": "Base"},
                                     {"id": "c", "name": "Base 2"}]});
    let names = ["Base".to_string()];
    assert_eq!(stale(&drawing, &names, false), [json!("b")]);
    assert_eq!(stale(&drawing, &names, true), [json!("a"), json!("b")]);
    assert!(stale(&json!({}), &names, false).is_empty());
}

#[test]
fn fills_in_the_ids_kept_earlier() {
    let ids = BTreeMap::from([
        ("sheet".to_string(), json!("s1")),
        ("front".to_string(), json!("v1")),
    ]);
    let args = json!({"sheet": "$sheet", "a": ["$front", "$other", "x", 3], "b": {"c": "$sheet"}});
    assert_eq!(
        fill(&args, &ids),
        json!({"sheet": "s1", "a": ["v1", "$other", "x", 3], "b": {"c": "s1"}})
    );
}

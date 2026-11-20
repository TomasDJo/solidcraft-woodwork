use super::*;

fn design(plinth: bool) -> Value {
    let mut params = vec![
        json!({"name": "A_width", "value": 600.0}),
        json!({"name": "A_thickness", "value": 18.0}),
    ];
    if plinth {
        params.push(json!({"name": "A_plinth", "value": 100.0}));
    }
    json!({ "params": params })
}

fn made(params: Value) -> Value {
    json!({"entry": "cabinet", "params": params})
}

/// The boxes made: (name, length, width, height, corner).
fn boxes(p: Value, cabinet: Value, plinth: bool) -> Result<Vec<(String, Value)>, String> {
    let (commands, _, _) = build(&p, "A", &cabinet, &[], &design(plinth))?;
    Ok(commands
        .into_iter()
        .filter(|c| c.0 == "solid.box")
        .map(|c| {
            let a = &c.1;
            let name = a["name"].as_str().unwrap_or_default().to_string();
            (
                name,
                json!([a["length"], a["width"], a["height"], a["corner"]]),
            )
        })
        .collect())
}

#[test]
fn end_panel_on_either_side_reaches_the_fronts() {
    // overlay doors: from the doors' faces to the back, floor to top, outside the side
    let overlay = made(json!({"doors": 2, "plinth": 100}));
    let right = boxes(
        json!({"kind": "end", "side": "right"}),
        overlay.clone(),
        true,
    )
    .unwrap();
    assert_eq!(right[0].0, "A End panel R");
    assert_eq!(
        right[0].1,
        json!([
            "A_thickness",
            "A_depth - (-A_reveal - A_thickness)",
            "A_height",
            [
                "A_width/2 + 0 mm",
                "(-A_reveal - A_thickness) - A_depth/2",
                "0 mm"
            ]
        ])
    );
    let left = boxes(
        json!({"kind": "end", "side": "left", "thickness": 19}),
        overlay,
        true,
    );
    let left = left.unwrap();
    assert_eq!(left[0].0, "A End panel L");
    assert_eq!(left[0].1[0], "19 mm");
    assert_eq!(left[0].1[3][0], "-A_width/2 - (0 mm) - (19 mm)");
    // inset fronts, or none: flush with the carcass
    for params in [json!({"doors": 1, "fronts": "inset"}), json!({})] {
        let flush = boxes(json!({"kind": "end", "side": "right"}), made(params), false);
        let flush = flush.unwrap();
        assert_eq!(flush[0].1[1], "A_depth - 0 mm");
        assert_eq!(flush[0].1[3][1], "0 mm - A_depth/2");
    }
}

#[test]
fn filler_and_its_return() {
    let overlay = made(json!({"drawers": 3}));
    let p = json!({"kind": "filler", "side": "left", "width": 60});
    let both = boxes(p, overlay, false).unwrap();
    assert_eq!(both[0].0, "A Filler L");
    assert_eq!(both[1].0, "A Filler L return");
    // the strip in the plane of the fronts, as high as they are
    assert_eq!(
        both[0].1,
        json!([
            "60 mm",
            "A_thickness",
            "A_height - 0 mm - 2*A_reveal",
            [
                "-A_width/2 - (0 mm) - (60 mm)",
                "(-A_reveal - A_thickness) - A_depth/2",
                "0 mm + A_reveal"
            ]
        ])
    );
    // the return behind its outer edge, back to the wall
    assert_eq!(both[1].1[0], "A_thickness");
    assert_eq!(
        both[1].1[1],
        "A_depth - (-A_reveal - A_thickness) - A_thickness"
    );
    assert_eq!(
        both[1].1[3][0],
        "-A_width/2 - (60 mm - A_thickness) - (A_thickness)"
    );
    // inset, on a plinth: the carcass above the plinth
    let inset = made(json!({"doors": 1, "fronts": "inset", "plinth": 100}));
    let p = json!({"kind": "filler", "side": "right", "units": "in", "width": 2});
    let both = boxes(p, inset, true).unwrap();
    assert_eq!(both[0].1[0], "2 in");
    assert_eq!(both[0].1[2], "A_height - A_plinth");
    assert_eq!(
        both[0].1[3],
        json!(["A_width/2 + 0 mm", "0 mm - A_depth/2", "A_plinth"])
    );
    // narrower than thick
    let p = json!({"kind": "filler", "side": "right", "width": 15});
    let err = boxes(p, made(json!({})), false).unwrap_err();
    assert!(err.contains("wider than its thickness"), "{err}");
}

#[test]
fn an_end_panel_ignores_the_filler_width() {
    // inches, and the dialog's 50 would be a 1270 mm filler
    let p = json!({"kind": "end", "side": "right", "units": "in", "width": 50, "thickness": 0.75});
    let one = boxes(p, made(json!({})), false).unwrap();
    assert_eq!(one[0].1[0], "0.75 in");
}

#[test]
fn refuses_corners_duplicates_and_bad_input() {
    let p = json!({"kind": "end", "side": "left"});
    let corner = json!({"entry": "corner", "params": {}});
    let err = build(&p, "A", &corner, &[], &design(false)).unwrap_err();
    assert!(err.contains("is a corner cabinet"), "{err}");
    let err = build(&p, "A", &Value::Null, &[], &design(false)).unwrap_err();
    assert!(err.contains("does not remember"), "{err}");
    let parts = ["A Side L".to_string(), "A End panel L".to_string()];
    let err = build(&p, "A", &made(json!({})), &parts, &design(false)).unwrap_err();
    assert_eq!(err, "`A` already has an end panel on its left");
    // one panel a side, of any kind; the other side is free
    let filler = json!({"kind": "filler", "side": "left"});
    let err = build(&filler, "A", &made(json!({})), &parts, &design(false)).unwrap_err();
    assert_eq!(err, "`A` already has an end panel on its left");
    let right = json!({"kind": "end", "side": "right"});
    assert!(build(&right, "A", &made(json!({})), &parts, &design(false)).is_ok());
    for bad in [
        json!({"kind": "shelf", "side": "left"}),
        json!({"kind": "plinth", "side": "left"}),
        json!({"kind": "end", "side": "auto"}),
        json!({"kind": "end", "side": "left", "thickness": 2}),
        json!({"kind": "filler", "side": "left", "width": 5}),
    ] {
        assert!(build(&bad, "A", &made(json!({})), &[], &design(false)).is_err());
    }
}

#[test]
fn record_folder_and_bodies() {
    let p = json!({"kind": "filler", "side": "right", "width": 50, "thickness": 0});
    let (commands, record, bodies) = build(&p, "A", &made(json!({})), &[], &design(false)).unwrap();
    assert_eq!(bodies, ["A Filler R", "A Filler R return"]);
    let group = commands.last().unwrap();
    assert_eq!(group.0, "timeline.group");
    assert_eq!(group.1, json!({"name": "A Filler R", "features": bodies}));
    assert_eq!(
        record,
        json!({"kind": "filler", "side": "right", "units": "mm", "width": 50.0, "thickness": 0.0})
    );
    assert_eq!(body("A", &record), "A Filler R");
}

#[test]
fn side_by_the_picked_board_and_place_refused_there() {
    let auto = json!({"side": "auto"});
    assert_eq!(side(&auto, "Side L"), Ok("left"));
    assert_eq!(side(&auto, "Side R"), Ok("right"));
    assert!(side(&auto, "Top")
        .unwrap_err()
        .contains("pick the cabinet's Side L"));
    assert_eq!(side(&json!({"side": "right"}), "Top"), Ok("right"));
    assert_eq!(side(&json!({}), "Side R"), Ok("right"));
    let parts = ["A Side L".to_string(), "A Filler R".to_string()];
    assert!(blocks(&parts, "A", "left").is_ok());
    let err = blocks(&parts, "A", "right").unwrap_err();
    assert!(err.starts_with("`A` has a filler on its right"), "{err}");
}

use super::*;

#[test]
fn only_the_missing_parameters_are_made() {
    let all = parameters(&json!({}));
    assert_eq!(all.len(), PARAMETERS.len());
    // every length carries its unit
    let slide = all.iter().find(|c| c.1["name"] == "HW_slide_t").unwrap();
    assert_eq!(slide.1["expression"], "12.7 mm");
    let design = json!({"params": [{"name": "HW_cup_d"}, {"name": "Cabinet_width"}]});
    let some = parameters(&design);
    assert_eq!(some.len(), PARAMETERS.len() - 1);
    assert!(some.iter().all(|c| c.1["name"] != "HW_cup_d"));
}

#[test]
fn a_cabinet_uses_them_with_fronts_only() {
    let built = |p: Value| crate::cabinet::build(&p).unwrap().0;
    assert!(!used(&built(json!({}))));
    assert!(used(&built(json!({"doors": 1}))));
    assert!(used(&built(json!({"drawers": 1}))));
}

#[test]
fn a_cup_goes_where_its_plate_is_on_the_pins_grid() {
    let z = on_grid("Z", "B");
    assert_eq!(
        z,
        "B + HW_pitch/2 + round((Z - (B) - HW_pitch/2) / HW_pitch) * HW_pitch"
    );
    // the same in numbers: base 118 (100 plinth, 18 bottom), cup wanted at 202 (a door from
    // 102, 100 up): 118 + 16 + round(68 / 32) * 32 = 198, plate holes at 182 and 214, pins
    // at 118 + 32k: 182 = 118 + 64 and 214 = 118 + 96
    let snapped = |z: f64, base: f64| base + 16.0 + ((z - base - 16.0) / 32.0).round() * 32.0;
    assert_eq!(snapped(202.0, 118.0), 198.0);
    assert!((snapped(202.0, 118.0) - 202.0).abs() <= 16.0);
}

#[test]
fn a_screw_hole_stops_short_of_a_thin_sides_outside() {
    assert_eq!(screw_depth("T"), "min(HW_screw_depth, T - 3 mm)");
    let (commands, _) = crate::cabinet::build(&json!({"drawers": 1, "thickness": 9})).unwrap();
    let hole = commands
        .iter()
        .find(|c| c.1["name"] == "Cabinet Side L Drawer 1 slide 1");
    assert_eq!(
        hole.unwrap().1["height"],
        "min(HW_screw_depth, Cabinet_thickness - 3 mm)"
    );
    assert!(crate::cabinet::build(&json!({"drawers": 1, "thickness": 5})).is_err());
}

fn copies<'a>(commands: &'a crate::Commands, model: &str) -> Vec<&'a Value> {
    let all = commands.iter().filter(|c| c.0 == "occurrence.copy");
    all.map(|c| &c.1)
        .filter(|a| a["component"] == model)
        .collect()
}

/// A copy's place: the translation column of its `_matrix`.
fn place(copy: &Value) -> Value {
    json!([0, 1, 2].map(|r| copy["_matrix"][r][3].clone()))
}

/// A copy's turn: the first three columns of its `_matrix`, by rows.
fn turn(copy: &Value) -> Vec<Vec<Value>> {
    (0..3)
        .map(|r| copy["_matrix"][r].as_array().unwrap()[..3].to_vec())
        .collect()
}

#[test]
fn a_hinge_model_sits_in_each_cup_with_its_plate_on_the_side() {
    let (commands, _) = crate::cabinet::build(&json!({"doors": 2})).unwrap();
    let hinges = copies(&commands, HINGE);
    assert_eq!(hinges.len(), 4);
    // in the door's own component, at the cup's centre on its back face
    for door in ["Door L", "Door R"] {
        let component = format!("Cabinet {door}");
        let cups = commands.iter().filter(|c| {
            c.0 == "solid.cylinder" && c.1["name"].as_str().unwrap().starts_with(&component)
        });
        let cups: Vec<&Value> = cups.map(|c| &c.1["base"]).collect();
        let ours: Vec<Value> = (hinges.iter())
            .filter(|h| h["parent"] == component.as_str())
            .map(|h| place(h))
            .collect();
        assert_eq!(cups.len(), 2);
        assert!(cups.iter().all(|c| ours.contains(c)), "{cups:?} {ours:?}");
    }
    // the left door's hinges as made (the arm towards -X, the hinge edge); the right door's
    // turned half about Y, so the arm reaches to its right edge
    let left = hinges
        .iter()
        .find(|h| h["parent"] == "Cabinet Door L")
        .unwrap();
    let right = hinges
        .iter()
        .find(|h| h["parent"] == "Cabinet Door R")
        .unwrap();
    let rows = |m: [[i32; 3]; 3]| m.map(|r| r.map(|v| json!(v)).to_vec()).to_vec();
    assert_eq!(turn(left), rows([[1, 0, 0], [0, 1, 0], [0, 0, 1]]));
    assert_eq!(turn(right), rows([[-1, 0, 0], [0, 1, 0], [0, 0, -1]]));
    assert!(hinges.iter().all(|h| h.get("_cut").is_none()));
    // a plate over each cup's two plate holes, in the cabinet, fixed; upright on the side
    let plates = copies(&commands, PLATE);
    assert_eq!(plates.len(), 4);
    assert!(plates.iter().all(|p| p["parent"] == "Cabinet"));
    let on_left: Vec<&&Value> = (plates.iter())
        .filter(|p| turn(p)[0][1] == json!(1))
        .collect();
    assert_eq!(on_left.len(), 2); // Side L: -Y into the side (-X), Z up
    assert_eq!(turn(on_left[0]), rows([[0, 1, 0], [-1, 0, 0], [0, 0, 1]]));
    // the holes are the same as without the models
    let off = crate::cabinet::build(&json!({"doors": 2, "hardware_models": "off"}));
    let rest = commands.into_iter().filter(|c| c.0 != "occurrence.copy");
    assert_eq!(rest.collect::<Vec<_>>(), off.unwrap().0);
    assert!(crate::cabinet::build(&json!({"hardware_models": "x"})).is_err());
}

#[test]
fn a_slide_model_on_each_side_as_long_as_the_slide() {
    let (commands, _) = crate::cabinet::build(&json!({"drawers": 2})).unwrap();
    // 560 deep: boxes 532 deep, the longest standard slide with 3 mm behind it is 500 mm
    let drawer = copies(&commands, "HW slide drawer 500 mm");
    let cabinet = copies(&commands, "HW slide cabinet 500 mm");
    let middle = copies(&commands, "HW slide middle 500 mm");
    assert_eq!((drawer.len(), cabinet.len(), middle.len()), (4, 4, 4));
    let in_drawer = |c: &&Value| c["parent"] == "Cabinet Drawer 1";
    assert!(drawer[..2].iter().all(in_drawer) && middle[..2].iter().all(in_drawer));
    assert!(cabinet.iter().all(|c| c["parent"] == "Cabinet"));
    // against the box side's outer face (4 mm thick), the carcass side's inner face (6 mm),
    // and the middle member between them
    assert_eq!(
        place(drawer[0])[0],
        "Cabinet_thickness + HW_slide_t - 2 mm - Cabinet_width/2"
    );
    assert_eq!(
        place(cabinet[1])[0],
        "Cabinet_width - Cabinet_thickness - 3 mm - Cabinet_width/2"
    );
    assert_eq!(
        place(middle[0])[0],
        "Cabinet_thickness + (HW_slide_t/2 + 1 mm) - Cabinet_width/2"
    );
    // full extension pulls out the slide's length, partial three quarters, over-travel 25 mm
    // more; a 2-section slide has no middle member
    let limit = |p: Value| {
        let (commands, _) = crate::cabinet::build(&p).unwrap();
        let limits = commands.iter().find(|c| c.0 == "joint.limits").unwrap();
        let middles = copies(&commands, "HW slide middle 500 mm").len();
        (limits.1["max"].clone(), middles)
    };
    assert_eq!(limit(json!({"drawers": 1})), (json!(500.0), 2));
    let partial = json!({"drawers": 1, "slide_type": "partial"});
    assert_eq!(limit(partial), (json!(375.0), 0));
    let over = json!({"drawers": 1, "slide_type": "over"});
    assert_eq!(limit(over), (json!(525.0), 2));
    assert!(crate::cabinet::build(&json!({"drawers": 1, "slide_type": "x"})).is_err());
    // the screw holes go by the slide's length; the box's carry it and the type in their names
    let hole = |name: &str| commands.iter().find(|c| c.1["name"] == name).unwrap();
    let back = hole("Cabinet Drawer 1 Left slide 2 (500 mm, full extension)");
    assert!(back.1["base"][1]
        .as_str()
        .unwrap()
        .contains("floor((500 mm - 2*HW_setback) / HW_pitch) * HW_pitch"));
    hole("Cabinet Side L Drawer 1 slide 2");
    // a definition of each member a length used
    assert_eq!(
        wanted(&commands),
        [
            "HW slide cabinet 500 mm",
            "HW slide drawer 500 mm",
            "HW slide middle 500 mm"
        ]
    );
    let made = definitions(&json!({}), &wanted(&commands));
    let bar = made
        .iter()
        .find(|c| c.1["body_name"] == "HW slide drawer 500 mm");
    assert_eq!(bar.unwrap().1["width"], "500 mm");
}

#[test]
fn the_slide_length_is_a_standard_one_that_fits() {
    // auto: the longest that fits with 3 mm behind it, in the cabinet's unit
    assert_eq!(slide_length("auto", "mm", 532.0), Ok((500.0, "500 mm")));
    assert_eq!(slide_length("auto", "mm", 553.0), Ok((550.0, "550 mm")));
    assert_eq!(slide_length("auto", "mm", 275.0), Ok((270.0, "270 mm")));
    assert_eq!(slide_length("auto", "in", 532.0), Ok((508.0, "20 in")));
    assert!(slide_length("auto", "mm", 252.0)
        .unwrap_err()
        .contains("no standard slide fits"));
    assert!(slide_length("auto", "in", 256.0).is_err());
    // one chosen: any standard length that fits, either unit
    assert_eq!(slide_length("450 mm", "in", 532.0), Ok((450.0, "450 mm")));
    assert_eq!(slide_length("18 in", "mm", 532.0), Ok((457.2, "18 in")));
    let e = slide_length("550 mm", "mm", 532.0).unwrap_err();
    assert!(e.contains("does not fit drawer boxes 532 mm deep"), "{e}");
    assert!(slide_length("460 mm", "mm", 532.0).is_err());
    // through the cabinet: Edit's field too
    let p = json!({"drawers": 1, "slide_length": "400 mm"});
    let (commands, _) = crate::cabinet::build(&p).unwrap();
    assert_eq!(copies(&commands, "HW slide drawer 400 mm").len(), 2);
    assert_eq!(slide_length("auto", "in", 720.0), Ok((711.2, "28 in")));
    assert_eq!(slide_length("auto", "mm", 700.0), Ok((650.0, "650 mm")));
    let p = json!({"drawers": 1, "depth": 400, "slide_length": "500 mm"});
    assert!(crate::cabinet::build(&p).is_err());
}

#[test]
fn a_corner_doors_hinges_are_turned_with_their_door() {
    let (commands, _) = crate::corner::build(&json!({"doors": "hinged left"})).unwrap();
    let hinges = copies(&commands, HINGE);
    assert!(!hinges.is_empty());
    // the return's door faces along X: the model's -Y turned onto its `into`, Z up or down
    let turned = hinges.iter().find(|h| h["_matrix"][0][0] == 0).unwrap();
    assert_eq!(turn(turned)[0][1].as_i64().unwrap().abs(), 1);
    assert_eq!(turn(turned)[2][2].as_i64().unwrap().abs(), 1);
    // the first door's plates in the end it hangs on
    assert!(!copies(&commands, PLATE).is_empty());
}

#[test]
fn with_cut_the_models_cut_the_cups() {
    let p = json!({"doors": 1, "drawers": 1, "hardware_models": "cut"});
    let (commands, _) = crate::cabinet::build(&p).unwrap();
    let cylinders = commands.iter().filter(|c| c.0 == "solid.cylinder");
    assert!(cylinders
        .clone()
        .all(|c| !c.1["name"].as_str().unwrap().contains("hinge cup")));
    let hinges = copies(&commands, HINGE);
    assert_eq!(hinges[0]["_cut"], "Cabinet Door hinge cup 1");
    // the plates and slides cut nothing: their screw holes stay
    assert!(copies(&commands, PLATE)[0].get("_cut").is_none());
    assert!(cylinders
        .clone()
        .any(|c| c.1["name"] == "Cabinet Side L Drawer 1 slide 1"));
}

#[test]
fn a_half_overlay_hinge_cuts_cups_named_so() {
    // the middle of three overlay doors hangs on a divider: its cups, drilled or cut by the
    // model, say half overlay, for the cut list's hardware (#157)
    let p = json!({"width": 900, "dividers": 2, "doors": 3});
    let (drilled, _) = crate::cabinet::build(&p).unwrap();
    assert!(drilled
        .iter()
        .any(|c| c.1["name"] == "Cabinet Door 2 hinge cup 1 (half overlay)"));
    let p = json!({"width": 900, "dividers": 2, "doors": 3, "hardware_models": "cut"});
    let (cut, _) = crate::cabinet::build(&p).unwrap();
    let hinges = copies(&cut, &hinge(HALF_OVERLAY));
    assert_eq!(
        hinges[0]["_cut"],
        "Cabinet Door 2 hinge cup 1 (half overlay)"
    );
}

#[test]
fn the_models_are_counted_against_the_command_limit() {
    let (commands, _) = crate::cabinet::build(&json!({"doors": 1})).unwrap();
    let n = commands.len();
    // two hinges and their plates, moved after they are copied; the definitions and back
    let made = definitions(&json!({}), &[HINGE.into(), PLATE.into()]).len() + 2;
    assert_eq!(cost(&commands), n + 4 + PARAMETERS.len() + made);
    let (cut, _) = crate::cabinet::build(&json!({"doors": 1, "hardware_models": "cut"})).unwrap();
    assert_eq!(cut.len(), n - 2);
    assert_eq!(cost(&cut), n - 2 + 4 + 4 + 1 + PARAMETERS.len() + made);
    // a cabinet the models would take over the limit is made without them
    let big = json!({"width": 2400, "height": 2400, "dividers": 7, "doors": 8, "shelves": 3,
                     "handles": "pull"});
    let mut off = big.clone();
    off["hardware_models"] = json!("off");
    let (with, summary) = crate::cabinet::build(&big).unwrap();
    assert_eq!(with, crate::cabinet::build(&off).unwrap().0);
    assert!(summary["hardware_models"]
        .as_str()
        .unwrap()
        .starts_with("left out"));
}

#[test]
fn the_definitions_are_made_once_at_the_root() {
    let all: Vec<String> = [
        HINGE,
        PLATE,
        "HW slide cabinet 450 mm",
        "HW slide drawer 450 mm",
    ]
    .map(String::from)
    .to_vec();
    let commands = definitions(&json!({"components": [{"name": "Untitled"}]}), &all);
    assert_eq!(
        commands[0],
        ("component.activate", json!({"component": "root"}))
    );
    assert_eq!(commands[1].1["name"], HOME);
    let made: Vec<&Value> = (commands.iter())
        .filter(|c| c.0 == "component.create")
        .map(|c| &c.1["name"])
        .collect();
    let mut names = vec![HOME.to_string()];
    names.extend(all.clone());
    assert_eq!(
        made,
        names
            .iter()
            .map(|n| json!(n))
            .collect::<Vec<_>>()
            .iter()
            .collect::<Vec<_>>()
    );
    let bodies: Vec<&Value> = commands.iter().map(|c| &c.1["body_name"]).collect();
    assert!(all.iter().all(|m| bodies.contains(&&json!(m))));
    // out of sight, under the floor, placed absolutely
    let moved = commands.last().unwrap();
    assert_eq!(moved.0, "occurrence.move");
    let to = &moved.1["to_matrix"];
    assert_eq!(to, &json!(HIDDEN));
    // under the floor, turned half round about Z, near the back of a cabinet on the origin
    const { assert!(HIDDEN[11] < -24.0 && HIDDEN[0] == -1.0 && HIDDEN[5] == -1.0 && HIDDEN[7] > 200.0) };
    // all there: nothing; one missing (a new length): only that one, in the hardware component
    let have = |names: &[&str]| {
        let components: Vec<Value> = names.iter().map(|n| json!({"name": n})).collect();
        json!({ "components": components })
    };
    let tree = have(&[
        HOME,
        HINGE,
        PLATE,
        "HW slide cabinet 450 mm",
        "HW slide drawer 450 mm",
    ]);
    assert!(definitions(&tree, &all).is_empty());
    let commands = definitions(&tree, &["HW slide drawer 300 mm".into()]);
    // a definition added later is made with `HOME` on the origin (a body made in a moved
    // component lands shifted in a plugin run), and `HOME` goes under the floor again after
    let origin = json!(ORIGIN);
    assert_eq!(commands[1].0, "occurrence.move");
    assert_eq!(commands[1].1["to_matrix"], origin);
    assert_eq!(
        commands[2],
        ("component.activate", json!({"component": HOME}))
    );
    assert_eq!(commands[3].1["name"], "HW slide drawer 300 mm");
    let last = commands.last().unwrap();
    assert_eq!(last.1["to_matrix"], json!(HIDDEN));
}

#[test]
fn the_models_are_hidden_from_every_reader() {
    assert!(is_model("HW hinge") && is_model("HW hinge plate (Base:1/HW hinge plate:3)"));
    assert!(is_model(
        "HW slide drawer 450 mm (Base:1/Base Drawer 1:1/HW slide drawer 450 mm:2)"
    ));
    assert!(!is_model("Base Door L") && !is_model("HW_cup_d"));
    let mut measure = json!({"bodies": [{"name": "Base Side L"}, {"name": "HW hinge (Base:1/HW hinge:2)"}, {"name": "HW hinge"}]});
    hide(&mut measure);
    assert_eq!(measure, json!({"bodies": [{"name": "Base Side L"}]}));
}

#[test]
fn a_copys_name_in_the_world() {
    // from component.list: the root (0), Room, Cab in it, its door, and the copy in the door
    let tree = json!({"components": [
        {"id": 0, "name": "Untitled", "occurrences": []},
        {"id": 6, "name": "Room", "occurrences": [{"id": 7, "name": "Room:1", "parent": 0}]},
        {"id": 8, "name": "Cab", "occurrences": [{"id": 9, "name": "Cab:1", "parent": 6}]},
        {"id": 10, "name": "Cab Door", "occurrences": [{"id": 11, "name": "Cab Door:1", "parent": 8}]},
        {"id": 3, "name": "HW hinge", "occurrences": [{"id": 4, "name": "HW hinge:1", "parent": 1},
            {"id": 14, "name": "HW hinge:3", "parent": 10}]},
    ]});
    assert_eq!(
        world_name(&tree, &json!(14), HINGE).unwrap(),
        "HW hinge (Room:1/Cab:1/Cab Door:1/HW hinge:3)"
    );
    assert_eq!(world_name(&tree, &json!(99), HINGE), None);
}

#[test]
fn a_door_takes_hinges_by_its_height() {
    let n = |h: f64| crate::fittings::cup_heights("A", "B", h).len();
    assert_eq!(
        [
            n(700.0),
            n(900.0),
            n(901.0),
            n(1600.0),
            n(1601.0),
            n(2000.0),
            n(2100.0)
        ],
        [2, 2, 3, 3, 4, 4, 5]
    );
    // the ends first, as before, then evenly between them
    let cups = crate::fittings::cup_heights("A", "B", 1700.0);
    assert_eq!(cups[..2], ["A + 100 mm", "B - 100 mm"]);
    assert_eq!(cups[2], "A + 100 mm + (B - (A) - 2*100 mm) * 1 / 3");
}

#[test]
fn the_hinge_angle_is_the_doors_limit() {
    let limits = |p: Value| -> Vec<Value> {
        let (commands, _) = crate::cabinet::build(&p).unwrap();
        let all = commands.iter().filter(|c| c.0 == "joint.limits");
        all.map(|c| c.1["max"].clone()).collect()
    };
    assert_eq!(limits(json!({"doors": 2})), [json!(110.0), json!(110.0)]);
    assert_eq!(
        limits(json!({"doors": 1, "hinge_angle": "165"})),
        [json!(165.0)]
    );
    assert!(crate::cabinet::build(&json!({"doors": 1, "hinge_angle": "120"})).is_err());
    // a corner's first door opens as far as its hinge; the fold always lays flat, 90
    let (commands, _) =
        crate::corner::build(&json!({"doors": "hinged left", "hinge_angle": "95"})).unwrap();
    let all = commands.iter().filter(|c| c.0 == "joint.limits");
    let max: Vec<Value> = all.map(|c| c.1["max"].clone()).collect();
    assert_eq!(max, [json!(95.0), json!(90.0)]);
}

#[test]
fn the_arm_is_cranked_for_inset_doors_and_dividers() {
    let models = |p: Value| -> Vec<String> {
        let (commands, _) = crate::cabinet::build(&p).unwrap();
        let mut found = wanted(&commands);
        found.retain(|m| m.starts_with(HINGE) && m != PLATE);
        found
    };
    assert_eq!(models(json!({"doors": 2})), [HINGE]);
    assert_eq!(
        models(json!({"doors": 2, "fronts": "inset"})),
        ["HW hinge crank 18"]
    );
    // three doors: the middle one hangs on a divider, half overlay
    let three = json!({"width": 900, "dividers": 2, "doors": 3});
    assert_eq!(models(three), [HINGE.to_string(), hinge(9.5)]);
    // the arm lies on the plate's outer face: a straight one 2 to 13 mm away from the hinge
    // edge, a cranked one 18 mm further towards it; the plate 8.5 off the side, 40 deep
    let arm = |model: &str| {
        let made = shape(model);
        let arm = &made[1].1;
        (
            arm["corner"][0].clone(),
            arm["length"].clone(),
            arm["width"].clone(),
        )
    };
    assert_eq!(arm(HINGE), (json!("2 mm"), json!("11 mm"), json!("53 mm")));
    assert_eq!(arm("HW hinge crank 18").0, json!("-16 mm"));
    let plate = &shape(PLATE)[0].1;
    assert_eq!(
        (&plate["length"], &plate["width"]),
        (&json!("40 mm"), &json!("8.5 mm"))
    );
    // recognisable (#140): the cup has a 40 mm rim and a boss where the arm starts, outside
    // the door (+Y), joined into one body; the plate is a cross, its wings 48 mm high
    let parts = |model: &str| -> Vec<String> {
        let made = shape(model);
        made.iter()
            .map(|c| c.1["name"].as_str().unwrap().to_string())
            .collect()
    };
    assert_eq!(
        parts(HINGE),
        [
            "HW hinge cup",
            "HW hinge arm",
            "HW hinge rim",
            "HW hinge boss"
        ]
    );
    let made = shape(HINGE);
    assert_eq!(
        (&made[2].1["diameter"], &made[2].1["axis"]),
        (&json!("40 mm"), &json!([0, 1, 0]))
    );
    assert_eq!(made[3].1["corner"][1], "0.1 mm");
    assert!(made[1..].iter().all(|c| c.1["targets"] == json!([HINGE])));
    assert_eq!(parts(PLATE), ["HW hinge plate", "HW hinge plate wings"]);
    let plate = shape(PLATE);
    assert_eq!(
        (&plate[0].1["height"], &plate[1].1["height"]),
        (&json!("16 mm"), &json!("48 mm"))
    );
}

#[test]
fn a_corners_fold_has_cups_but_no_hinge_models() {
    // the first door's cups each get a hinge; the second door's (the fold's) only holes
    for doors in ["hinged left", "hinged right"] {
        let (commands, _) = crate::corner::build(&json!({ "doors": doors })).unwrap();
        let cups = |door: &str| {
            let all = commands.iter().filter(|c| c.0 == "solid.cylinder");
            let start = format!("Cabinet {door} hinge cup ");
            all.filter(|c| c.1["name"].as_str().unwrap().starts_with(&start))
                .count()
        };
        let hinges = |door: &str| {
            let all = copies(&commands, HINGE);
            let parent = format!("Cabinet {door}");
            all.iter()
                .filter(|h| h["parent"] == parent.as_str())
                .count()
        };
        let (first, fold) = match doors {
            // a left corner hinged left hangs on its return's end
            "hinged left" => ("Door return", "Door"),
            _ => ("Door", "Door return"),
        };
        assert!(cups(first) == 2 && hinges(first) == 2, "{doors}");
        assert!(cups(fold) == 2 && hinges(fold) == 0, "{doors}");
    }
}

#[test]
fn undermount_lengths_fit_the_inside_depth() {
    // 560 deep, a groove back: 542 inside; 3 mm behind the slide
    assert_eq!(under_length("auto", "mm", 542.0), Ok((500.0, "500 mm")));
    assert_eq!(under_length("auto", "in", 542.0), Ok((533.4, "21 in")));
    assert_eq!(under_length("15 in", "mm", 542.0), Ok((381.0, "15 in")));
    assert!(under_length("650 mm", "mm", 700.0).is_err()); // side-mount only
    assert!(slide_length("21 in", "in", 700.0).is_err()); // undermount only
    assert!(under_length("auto", "mm", 240.0)
        .unwrap_err()
        .contains("250 mm"));
    assert_eq!(travel("under_partial", 500.0), 375.0);
    assert_eq!(travel("under_full", 500.0), 500.0);
}

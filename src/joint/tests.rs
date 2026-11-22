use super::*;

fn find<'a>(commands: &'a Commands, name: &str) -> &'a Value {
    let found = commands.iter().find(|c| c.1["name"] == name);
    &found.unwrap_or_else(|| panic!("no {name}")).1
}

/// The pattern that repeats `feature`.
fn row<'a>(commands: &'a Commands, feature: &str) -> &'a Value {
    let found = (commands.iter())
        .filter(|c| c.0 == "solid.pattern.rectangular")
        .find(|c| {
            c.1["features"]
                .as_array()
                .unwrap()
                .contains(&json!(feature))
        });
    &found.unwrap_or_else(|| panic!("no row of {feature}")).1
}

fn named(commands: &Commands, part: &str) -> usize {
    (commands.iter())
        .filter(|c| c.1["name"].as_str().is_some_and(|n| n.contains(part)))
        .count()
}

#[test]
fn dowels_are_one_cut_through_both_boards_and_a_row_that_follows_the_depth() {
    let (commands, _) = crate::cabinet::build(&json!({"name": "K", "joints": "dowels"})).unwrap();
    let dowel = find(&commands, "K joint Side L Bottom dowels");
    assert_eq!(dowel["targets"], json!(["K Side L", "K Bottom"]));
    assert_eq!(dowel["axis"], json!([1, 0, 0]));
    assert_eq!(dowel["diameter"], "K_dowel_d");
    assert_eq!(dowel["height"], "K_dowel_length + 2 mm");
    // blind in the side's face, from its inner face at x = t
    let x = dowel["base"][0].as_str().unwrap();
    assert!(
        x.starts_with("K_thickness - (min(K_thickness - 5 mm"),
        "{x}"
    );
    let y = dowel["base"][1].as_str().unwrap();
    assert!(y.starts_with("0 mm + (K_joint_edge)"), "{y}");
    // one pattern for the four joints: the same way, the same count
    let row = row(&commands, "K joint Side R Top dowels");
    assert_eq!(row["name"], "K joint Side L Bottom dowels row");
    assert_eq!(row["features"].as_array().unwrap().len(), 4);
    assert_eq!(row["dir1"], json!([0, 1, 0]));
    let count = "max(2, ceil((K_depth - 2*K_joint_edge) / K_joint_spacing) + 1)";
    assert_eq!(row["count1"], count);
    assert_eq!(
        row["spacing1"],
        format!("(K_depth - 2*K_joint_edge) / ({count} - 1)")
    );
    assert_eq!(named(&commands, " row"), 1);
    // the sizes are parameters, and the dowels to buy are counted in one
    let added = |name: &str| {
        (commands.iter())
            .find(|c| c.0 == "parameters.add" && c.1["name"] == name)
            .map(|c| c.1["expression"].as_str().unwrap_or_default().to_string())
    };
    assert_eq!(added("K_dowel_d").as_deref(), Some("8 mm"));
    assert_eq!(added("K_joint_spacing").as_deref(), Some("250 mm"));
    assert_eq!(added("K_dowels"), Some([count; 4].join(" + ")));
    assert_eq!(added("K_confirmats"), None);
    // the holes come after every board
    let last_box = commands.iter().rposition(|c| c.0 == "solid.box").unwrap();
    let first_joint = (commands.iter())
        .position(|c| c.1["name"] == "K joint Side L Bottom dowels")
        .unwrap();
    assert!(first_joint > last_box);
}

#[test]
fn confirmat_is_a_clearance_hole_through_the_side_and_a_pilot_in_the_end() {
    let p = json!({"name": "K", "joints": "confirmat", "back": "overlay", "plinth": 100});
    let (commands, _) = crate::cabinet::build(&p).unwrap();
    let clear = find(&commands, "K joint Side R Bottom confirmat");
    assert_eq!(clear["targets"], json!(["K Side R"]));
    assert_eq!(clear["diameter"], "7 mm");
    assert_eq!(clear["axis"], json!([-1, 0, 0]));
    let x = clear["base"][0].as_str().unwrap();
    assert!(
        x.starts_with("K_width - K_thickness + (K_thickness)"),
        "{x}"
    );
    assert_eq!(clear["base"][2], "K_plinth + K_thickness/2");
    let pilot = find(&commands, "K joint Side R Bottom confirmat pilot");
    assert_eq!(pilot["targets"], json!(["K Bottom"]));
    assert_eq!(pilot["height"], "50 mm - K_thickness + 2 mm");
    let row = row(&commands, "K joint Side R Bottom confirmat pilot");
    assert_eq!(row["features"].as_array().unwrap().len(), 8);
    // an overlaid back: the sides stop in front of it
    let count = row["count1"].as_str().unwrap();
    assert!(count.contains("(K_depth - K_back_thickness)"), "{count}");
    assert!(named(&commands, "dowel") == 0);
    let thick = json!({"joints": "confirmat", "thickness": 40});
    assert!(crate::cabinet::build(&thick)
        .unwrap_err()
        .contains("50 mm screws"));
}

#[test]
fn a_sink_base_gets_one_fastener_in_the_middle_of_each_rail() {
    let p = json!({"name": "S", "kind": "sink", "joints": "dowels"});
    let (commands, _) = crate::cabinet::build(&p).unwrap();
    let front = find(&commands, "S joint Side L Rail front dowels");
    assert_eq!(front["targets"], json!(["S Side L", "S Rail front"]));
    let y = front["base"][1].as_str().unwrap();
    assert!(y.starts_with("0 mm + (100 mm / 2)"), "{y}");
    let back = find(&commands, "S joint Side R Rail back dowels");
    let y = back["base"][1].as_str().unwrap();
    assert!(
        y.contains("S_back_thickness) - 100 mm + (100 mm / 2)"),
        "{y}"
    );
    // the bottom's two joints as on any cabinet, in one pattern
    assert_eq!(named(&commands, " row"), 1);
    let dowels = (commands.iter())
        .find(|c| c.1["name"] == "S_dowels")
        .unwrap();
    let expression = dowels.1["expression"].as_str().unwrap();
    assert_eq!(expression.matches(") + 1 + 1").count(), 2, "{expression}");
}

#[test]
fn no_method_makes_nothing_and_an_edit_that_drops_one_zeroes_its_count() {
    let (plain, _) = crate::cabinet::build(&json!({"name": "K"})).unwrap();
    assert_eq!(named(&plain, " joint "), 0);
    assert!(!plain.iter().any(|c| c.1["name"] == "K_joint_spacing"));
    let (none, _) = crate::cabinet::build(&json!({"name": "K", "joints": "none"})).unwrap();
    assert_eq!(plain, none);
    let edited = json!({"name": "K", "joints": "confirmat",
        "_existing": {"dowels": 16, "joint_spacing": 250, "joint_edge": 50}});
    let (commands, _) = crate::cabinet::build(&edited).unwrap();
    let change = (commands.iter())
        .find(|c| c.0 == "parameters.change" && c.1["name"] == "K_dowels")
        .unwrap();
    assert_eq!(change.1["expression"], "0");
    // kept as they were: no command for the spacing and edge
    assert!(!commands.iter().any(|c| c.1["name"] == "K_joint_spacing"));
    assert!(commands
        .iter()
        .any(|c| c.0 == "parameters.add" && c.1["name"] == "K_confirmats"));
}

#[test]
fn sizes_are_checked() {
    let refused = |p: Value, word: &str| {
        let error = crate::cabinet::build(&p).unwrap_err();
        assert!(error.contains(word), "{error}");
    };
    refused(json!({"joints": "glue"}), "joints");
    refused(json!({"joints": "dowels", "dowel_d": 16}), "dowel");
    refused(
        json!({"joints": "dowels", "joint_spacing": 5}),
        "joint_spacing",
    );
    refused(
        json!({"joints": "dowels", "depth": 250, "joint_edge": 120}),
        "joint_edge",
    );
    // inches: the sizes in the dialog's unit
    let p = json!({"name": "I", "units": "in", "width": 24, "height": 30, "depth": 22,
        "thickness": 0.75, "joints": "dowels", "dowel_d": 0.3125, "dowel_length": 1.5});
    let (commands, _) = crate::cabinet::build(&p).unwrap();
    let d = commands
        .iter()
        .find(|c| c.1["name"] == "I_dowel_d")
        .unwrap();
    assert_eq!(d.1["expression"], "0.3125 in");
}

#[test]
fn the_cut_list_reads_the_counts_and_sizes_from_the_parameters() {
    let params = json!([
        {"name": "K_dowels", "value": 16.0}, {"name": "K_dowel_d", "value": 8.0},
        {"name": "K_dowel_length", "value": 35.0}, {"name": "K_confirmats", "value": 0.0},
        {"name": "L_confirmats", "value": 12.0}
    ]);
    assert_eq!(
        hardware("K", &params),
        vec![("Dowel, 8 x 35 mm".to_string(), 16)]
    );
    assert_eq!(
        hardware("L", &params),
        vec![("Confirmat screw, 7 x 50 mm".to_string(), 12)]
    );
    assert!(hardware("M", &params).is_empty());
}

#[test]
fn dividers_and_the_rail_take_the_second_method() {
    let p = json!({"name": "K", "width": 1200, "dividers": 2, "shelves": 0, "joints": "dowels",
        "joints_inner": "confirmat"});
    let (commands, _) = crate::cabinet::build(&p).unwrap();
    // a divider's end into the bottom's face: from under the bottom, up into the divider
    let clear = find(&commands, "K joint Bottom Divider 2 confirmat");
    assert_eq!(clear["targets"], json!(["K Bottom"]));
    assert_eq!(clear["axis"], json!([0, 0, 1]));
    assert_eq!(clear["base"][2], "0 mm + K_thickness - (K_thickness)");
    let pilot = find(&commands, "K joint Top Divider 1 confirmat pilot");
    assert_eq!(pilot["targets"], json!(["K Divider 1"]));
    assert_eq!(pilot["axis"], json!([0, 0, -1]));
    let row = row(&commands, "K joint Top Divider 1 confirmat");
    let count = row["count1"].as_str().unwrap();
    assert!(
        count.contains("(K_depth - 12 mm - K_back_thickness)"),
        "{count}"
    );
    // sides and dividers: two depths, two patterns
    assert_eq!(named(&commands, " row"), 2);
    let added = |name: &str| {
        commands
            .iter()
            .any(|c| c.0 == "parameters.add" && c.1["name"] == name)
    };
    assert!(added("K_dowels") && added("K_confirmats") && added("K_dowel_d"));
    // the rail between drawers and doors: like the bottom, behind the fronts' line
    let p = json!({"name": "M", "doors": 1, "drawers": 1, "joints": "none",
        "joints_inner": "dowels", "fronts": "inset"});
    let (commands, _) = crate::cabinet::build(&p).unwrap();
    let dowel = find(&commands, "M joint Side R Rail dowels");
    assert_eq!(dowel["targets"], json!(["M Side R", "M Rail"]));
    let y = dowel["base"][1].as_str().unwrap();
    assert!(y.starts_with("M_thickness + (M_joint_edge)"), "{y}");
    assert_eq!(named(&commands, " row"), 1);
    // no dividers, rail or shelves: the second method is not read
    let (plain, _) =
        crate::cabinet::build(&json!({"name": "K", "shelves": 0, "joints_inner": "glue"})).unwrap();
    assert_eq!(named(&plain, " joint "), 0);
}

#[test]
fn a_corner_cabinet_joins_its_backs_ends_bottom_and_top() {
    let names = |p: Value| -> Vec<String> {
        let (commands, _) = crate::corner::build(&p).unwrap();
        (commands.iter())
            .filter(|c| c.0 == "solid.cylinder")
            .filter_map(|c| c.1["name"].as_str().map(String::from))
            .filter(|n| n.contains(" joint "))
            .collect()
    };
    let l = names(json!({"name": "C", "joints": "dowels"}));
    // three upright joints, and four for each of the bottom and the top
    assert_eq!(l.len(), 3 + 8, "{l:?}");
    assert!(
        l.contains(&"C joint Side R Back dowels".to_string()),
        "{l:?}"
    );
    assert!(
        l.contains(&"C joint Back return Back dowels".to_string()),
        "{l:?}"
    );
    assert!(
        l.contains(&"C joint Side L Back return dowels".to_string()),
        "{l:?}"
    );
    assert!(l.contains(&"C joint Back Top dowels".to_string()), "{l:?}");
    // split: the return's piece meets the return's end and the back return
    let split = names(json!({"name": "C", "joints": "dowels", "boards": "split"}));
    assert_eq!(split.len(), 3 + 10, "{split:?}");
    assert!(split.contains(&"C joint Side L Bottom return dowels".to_string()));
    assert!(split.contains(&"C joint Back return Bottom return dowels".to_string()));
    // with a wall mount the backs stand on the bottom
    let mount = names(json!({"name": "C", "joints": "dowels", "wall_mount": "rail"}));
    assert!(
        mount.contains(&"C joint Bottom Back dowels".to_string()),
        "{mount:?}"
    );
    assert!(
        mount.contains(&"C joint Top Back return dowels".to_string()),
        "{mount:?}"
    );
    assert_eq!(mount.len(), 3 + 8);
    // a right corner: the same, mirrored
    let (commands, _) =
        crate::corner::build(&json!({"name": "C", "joints": "dowels", "corner": "right"})).unwrap();
    let back = find(&commands, "C joint Side L Back dowels");
    assert_eq!(back["axis"], json!([1, 0, 0]));
    let row = row(&commands, "C joint Back Bottom dowels");
    assert_eq!(row["dir1"], json!([-1, 0, 0]));
    assert!(names(json!({"name": "C"})).is_empty());
    let diagonal = names(json!({"name": "C", "joints": "confirmat", "shape": "diagonal"}));
    assert_eq!(diagonal.len(), 2 * (3 + 8));
}

#[test]
fn plain_screws_are_a_thinner_confirmat() {
    let p = json!({"name": "K", "joints": "screws"});
    let (commands, _) = crate::cabinet::build(&p).unwrap();
    let clear = find(&commands, "K joint Side L Top screws");
    assert_eq!(clear["targets"], json!(["K Side L"]));
    assert_eq!(clear["diameter"], "4.5 mm");
    let pilot = find(&commands, "K joint Side L Top screws pilot");
    assert_eq!(pilot["diameter"], "3 mm");
    assert!(commands
        .iter()
        .any(|c| c.0 == "parameters.add" && c.1["name"] == "K_screws"));
    let params = json!([{"name": "K_screws", "value": 12.0}]);
    assert_eq!(
        hardware("K", &params),
        vec![("Screw, 4 x 50 mm".to_string(), 12)]
    );
}

#[test]
fn a_dado_makes_the_board_longer_and_cuts_its_groove() {
    let p = json!({"name": "K", "dividers": 1, "joints": "dado", "joints_inner": "dado"});
    let (commands, _) = crate::cabinet::build(&p).unwrap();
    let bottom = find(&commands, "K Bottom");
    assert_eq!(
        bottom["length"],
        "(K_width - 2*K_thickness) + 2*K_dado_depth"
    );
    assert_eq!(
        bottom["corner"][0],
        "K_thickness - K_dado_depth - K_width/2"
    );
    // the groove in the side: the bottom's end, dado_depth deep
    let groove = find(&commands, "K joint Side L Bottom dado");
    assert_eq!(groove["targets"], json!(["K Side L"]));
    assert_eq!(groove["length"], "K_dado_depth");
    assert_eq!(groove["height"], "K_thickness");
    assert_eq!(
        groove["corner"][0],
        "K_thickness - K_dado_depth - K_width/2"
    );
    assert_eq!(groove["corner"][2], "0 mm + K_thickness/2 - K_thickness/2");
    let right = find(&commands, "K joint Side R Top dado");
    assert_eq!(right["corner"][0], "K_width - K_thickness - K_width/2");
    // a divider grows down into the bottom and up into the top
    let divider = find(&commands, "K Divider 1");
    assert_eq!(
        divider["height"],
        "(K_height - 0 mm - 2*K_thickness) + 2*K_dado_depth"
    );
    let groove = find(&commands, "K joint Bottom Divider 1 dado");
    assert_eq!(groove["height"], "K_dado_depth");
    assert_eq!(groove["targets"], json!(["K Bottom"]));
    assert!(!commands.iter().any(|c| c.0 == "solid.pattern.rectangular"));
    assert!(
        crate::cabinet::build(&json!({"joints": "dado", "dado_depth": 16}))
            .unwrap_err()
            .contains("dado_depth")
    );
    assert!(crate::corner::build(&json!({"joints": "dado"}))
        .unwrap_err()
        .contains("box cabinets"));
}

#[test]
fn a_biscuit_is_a_slot_half_in_each_board() {
    let p = json!({"name": "K", "joints": "biscuit", "biscuit": "10"});
    let (commands, _) = crate::cabinet::build(&p).unwrap();
    let slot = find(&commands, "K joint Side L Bottom biscuit");
    assert_eq!(slot["targets"], json!(["K Side L", "K Bottom"]));
    // across the joint 11 mm each way, 59 mm along it, 4 mm thick in the bottom's middle
    assert_eq!(
        [&slot["length"], &slot["width"], &slot["height"]],
        [&json!("22 mm"), &json!("59 mm"), &json!("4 mm")]
    );
    let x = slot["corner"][0].as_str().unwrap();
    assert_eq!(x, "K_thickness - 11 mm - K_width/2");
    let row = row(&commands, "K joint Side L Bottom biscuit");
    assert_eq!(row["features"].as_array().unwrap().len(), 4);
    assert!(commands.iter().any(|c| c.1["name"] == "K_biscuits_10"));
    let params =
        json!([{"name": "K_dominos", "value": 12.0}, {"name": "K_biscuits_20", "value": 2.0}]);
    assert_eq!(
        hardware("K", &params),
        vec![
            ("Biscuit, #20".to_string(), 2),
            ("Domino, 5 x 30 mm".to_string(), 12)
        ]
    );
    let thin = json!({"joints": "biscuit", "biscuit": "domino", "thickness": 9});
    assert!(crate::cabinet::build(&thin)
        .unwrap_err()
        .contains("too thin"));
}

#[test]
fn a_cam_lock_is_a_bolt_into_the_face_and_a_cam_in_the_end_boards_inside_face() {
    let p = json!({"name": "K", "joints": "cam"});
    let (commands, _) = crate::cabinet::build(&p).unwrap();
    let thread = find(&commands, "K joint Side L Bottom cam");
    assert_eq!(thread["targets"], json!(["K Side L"]));
    assert_eq!(thread["axis"], json!([-1, 0, 0]));
    let bolt = find(&commands, "K joint Side L Bottom cam bolt");
    assert_eq!(bolt["targets"], json!(["K Bottom"]));
    assert_eq!(
        bolt["height"],
        "K_cam_bolt - min(K_thickness - 5 mm, 11 mm)"
    );
    // the bottom's cam from its top face (inside), the top's from its underside
    let housing = find(&commands, "K joint Side L Bottom cam housing");
    assert_eq!(housing["axis"], json!([0, 0, -1]));
    assert_eq!(housing["diameter"], "K_cam_d");
    let z = housing["base"][2].as_str().unwrap();
    assert_eq!(z, "0 mm + K_thickness/2 + (K_thickness/2)");
    let top = find(&commands, "K joint Side R Top cam housing");
    assert_eq!(top["axis"], json!([0, 0, 1]));
    let x = top["base"][0].as_str().unwrap();
    assert!(
        x.contains("- (K_cam_bolt - min(K_thickness - 5 mm, 11 mm))"),
        "{x}"
    );
    assert_eq!(
        row(&commands, "K joint Side L Bottom cam housing")["features"]
            .as_array()
            .unwrap()
            .len(),
        12
    );
    let params = json!([{"name": "K_cams", "value": 12.0}, {"name": "K_cam_d", "value": 15.0}]);
    assert_eq!(
        hardware("K", &params),
        vec![("Cam lock and bolt, 15 mm".to_string(), 12)]
    );
    assert!(
        crate::cabinet::build(&json!({"joints": "cam", "cam_bolt": 20}))
            .unwrap_err()
            .contains("cam")
    );
}

#[test]
fn a_pocket_screw_goes_in_at_15_degrees_from_the_end_boards_outside_face() {
    let p = json!({"name": "K", "joints": "pocket"});
    let (commands, _) = crate::cabinet::build(&p).unwrap();
    // the bottom's pockets from underneath, toward the left side and up to its middle
    let pocket = find(&commands, "K joint Side L Bottom pocket");
    assert_eq!(pocket["axis"], json!([-0.965926, 0.0, 0.258819]));
    assert_eq!(pocket["targets"], json!(["K Bottom"]));
    assert_eq!(pocket["diameter"], "9.5 mm");
    let x = pocket["base"][0].as_str().unwrap();
    assert!(
        x.starts_with("K_thickness + (K_thickness / 2 / 0.258819 + 5 mm) * 0.965926"),
        "{x}"
    );
    let z = pocket["base"][2].as_str().unwrap();
    assert!(
        z.starts_with("0 mm + K_thickness/2 - (K_thickness / 2 / 0.258819 + 5 mm) * 0.258819"),
        "{z}"
    );
    let pilot = find(&commands, "K joint Side L Bottom pocket pilot");
    assert_eq!(pilot["targets"], json!(["K Bottom", "K Side L"]));
    // the top's from above, the right side's the other way
    let top = find(&commands, "K joint Side R Top pocket");
    assert_eq!(top["axis"], json!([0.965926, 0.0, -0.258819]));
    // inside: the bottom's from its top face
    let p = json!({"name": "K", "joints": "pocket", "pocket_face": "inside"});
    let (commands, _) = crate::cabinet::build(&p).unwrap();
    let pocket = find(&commands, "K joint Side L Bottom pocket");
    assert_eq!(pocket["axis"], json!([-0.965926, 0.0, -0.258819]));
    let params = json!([{"name": "K_pocket_screws", "value": 12.0}]);
    assert_eq!(
        hardware("K", &params),
        vec![("Pocket screw, 32 mm".to_string(), 12)]
    );
}

#[test]
fn without_pins_the_shelves_are_fixed_and_joined_to_the_sides_and_dividers() {
    let p = json!({"name": "K", "width": 1200, "dividers": 1, "shelves": 2,
        "joints": "none", "joints_inner": "dowels"});
    let (commands, _) = crate::cabinet::build(&p).unwrap();
    assert_eq!(named(&commands, " pins"), 0);
    // shelf 2.1: its left end into the divider's right face, between the holes of shelf 1.1
    let left = find(&commands, "K joint Divider 1 Shelf 2.1 dowels");
    assert_eq!(left["axis"], json!([1, 0, 0]));
    let y = left["base"][1].as_str().unwrap();
    assert!(y.contains("K_joint_edge + ("), "{y}");
    let other = find(&commands, "K joint Divider 1 Shelf 1.1 dowels");
    assert_eq!(other["axis"], json!([-1, 0, 0]));
    assert!(
        find(&commands, "K joint Side R Shelf 2.2 dowels")["targets"]
            == json!(["K Side R", "K Shelf 2.2"])
    );
    let between = row(&commands, "K joint Divider 1 Shelf 2.1 dowels");
    assert!(between["count1"].as_str().unwrap().ends_with("- 1)"));
    // dividers (2 ends) and shelves (2 ends each of 4)
    let dowels = (commands.iter())
        .find(|c| c.1["name"] == "K_dowels")
        .unwrap();
    assert_eq!(
        dowels.1["expression"]
            .as_str()
            .unwrap()
            .matches("max(2")
            .count(),
        2 + 8
    );
    // a dado: the shelf as long again as the grooves
    let p = json!({"name": "K", "shelves": 1, "joints_inner": "dado"});
    let (commands, _) = crate::cabinet::build(&p).unwrap();
    let shelf = find(&commands, "K Shelf 1");
    assert!(shelf["length"]
        .as_str()
        .unwrap()
        .ends_with("+ 2*K_dado_depth"));
    assert_eq!(
        find(&commands, "K joint Side L Shelf 1 dado")["targets"],
        json!(["K Side L"])
    );
    let deep = json!({"dividers": 1, "joints_inner": "dado", "dado_depth": 8});
    assert!(crate::cabinet::build(&deep)
        .unwrap_err()
        .contains("dado_depth"));
    // on pins: adjustable, no joints
    let p = json!({"name": "K", "shelf_pins": true, "joints_inner": "dowels"});
    let (commands, _) = crate::cabinet::build(&p).unwrap();
    assert!(named(&commands, " pins") > 0 && named(&commands, " joint ") == 0);
}

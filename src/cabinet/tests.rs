use super::*;

fn count(commands: &[(&str, Value)], id: &str) -> usize {
    commands.iter().filter(|c| c.0 == id).count()
}

#[test]
fn default_cabinet() {
    let (commands, summary) = build(&json!({})).unwrap();
    // width, height, depth, thickness, back thickness
    assert_eq!(count(&commands, "parameters.add"), 5);
    assert_eq!(commands[0].1["name"], "Cabinet_width");
    assert_eq!(commands[0].1["expression"], "600 mm");
    // 2 sides, bottom, top, 1 shelf, groove cut, back
    assert_eq!(count(&commands, "solid.box"), 7);
    assert_eq!(summary["bodies"].as_array().unwrap().len(), 6);
    assert_eq!(summary["inner"]["width"], 564.0);
    // every size is an expression over the parameters
    let side = &commands.iter().find(|c| c.0 == "solid.box").unwrap().1;
    assert_eq!(side["height"], "Cabinet_height");
    assert_eq!(side["corner"][0], "0 mm - Cabinet_width/2");
    // the history folder holds every feature
    let group = &commands.last().unwrap().1;
    assert_eq!(group["features"].as_array().unwrap().len(), 9);
    // the two seats, where a neighbour stands: the back corners, by the parameters
    let seats: Vec<&Value> = (commands.iter())
        .filter(|c| c.0 == "construct.axis.two_planes")
        .map(|c| &c.1)
        .collect();
    assert_eq!(seats.len(), 2);
    assert_eq!(seats[1]["name"], "Cabinet seat R");
    let corner = [
        &seats[1]["a"]["plane"]["origin"][0],
        &seats[1]["b"]["plane"]["origin"][1],
    ];
    assert_eq!(
        corner,
        [
            "Cabinet_width - Cabinet_width/2",
            "Cabinet_depth - Cabinet_depth/2"
        ]
    );
}

#[test]
fn inches_stay_inches() {
    let p = json!({"units": "in", "width": 24, "height": 30, "depth": 22,
                   "thickness": 0.75, "back_thickness": 0.25, "name": "Base 1"});
    let (commands, summary) = build(&p).unwrap();
    assert_eq!(commands[0].1["name"], "Base_1_width");
    assert_eq!(commands[0].1["expression"], "24 in");
    assert_eq!(summary["inner"]["width"], 24.0 * 25.4 - 2.0 * 19.05);
    // the reveal is mm whatever the units: the default 2 builds, as 2 mm
    let p = json!({"units": "in", "width": 24, "height": 30, "depth": 22,
                   "thickness": 0.75, "doors": 2, "reveal": 2});
    let (commands, _) = build(&p).unwrap();
    let reveal = commands.iter().find(|c| c.1["name"] == "Cabinet_reveal");
    assert_eq!(reveal.unwrap().1["expression"], "2 mm");
}

#[test]
fn the_auto_handle_goes_by_the_door_height() {
    // inset in 1240: the door is 1240 - 2 x (18 + 2) = 1200, not over 1200, so a floor
    // unit's knob goes near the top, not in the middle
    let p = json!({"fronts": "inset", "doors": 1, "height": 1240, "handles": "knob"});
    let (commands, _) = build(&p).unwrap();
    let hole = commands
        .iter()
        .find(|c| c.1["name"] == "Cabinet Door handle 1");
    assert_eq!(
        hole.unwrap().1["base"][2],
        "Cabinet_height - Cabinet_thickness - Cabinet_reveal - 100 mm + 0 mm"
    );
}

#[test]
fn options_add_parts() {
    let p = json!({"back": "overlay", "shelves": 0, "plinth": 100});
    let (commands, _) = build(&p).unwrap();
    assert_eq!(count(&commands, "parameters.add"), 7); // with the plinth and its set-back
    assert_eq!(count(&commands, "solid.box"), 6); // 4 panels, plinth, back

    let p = json!({"shelf_pins": true, "doors": 2, "height": 1200});
    let (commands, summary) = build(&p).unwrap();
    assert_eq!(count(&commands, "solid.pattern.rectangular"), 2);
    // pin seeds, 3 cups per tall door and a plate's 2 screw holes beside each
    assert_eq!(count(&commands, "solid.cylinder"), 2 + 2 * 3 * 3);
    assert_eq!(summary["bodies"].as_array().unwrap().len(), 8);
}

#[test]
fn plinth_types() {
    let named = |commands: &[(&str, Value)], name: &str| -> Value {
        let found = commands.iter().find(|c| c.1["name"] == name);
        found.map(|c| c.1.clone()).unwrap_or_default()
    };
    // notched sides: no board, a toe kick cut out of each side's front bottom corner
    let p = json!({"units": "in", "width": 24, "height": 34.5, "depth": 22, "thickness": 0.75,
                   "back_thickness": 0.25, "plinth": 4, "plinth_type": "notch"});
    let (commands, summary) = build(&p).unwrap();
    assert!(named(&commands, "Cabinet Plinth").is_null());
    assert!(!summary["bodies"].to_string().contains("Plinth"));
    let notch = named(&commands, "Cabinet Side R notch");
    assert_eq!(notch["operation"], "cut");
    assert_eq!(notch["targets"], json!(["Cabinet Side R"]));
    assert_eq!(
        [&notch["length"], &notch["width"], &notch["height"]],
        [
            "Cabinet_thickness",
            "Cabinet_plinth_setback",
            "Cabinet_plinth"
        ]
    );
    assert_eq!(notch["corner"][2], "0 mm");
    let setback = named(&commands, "Cabinet_plinth_setback");
    assert_eq!(setback["expression"], "1.968504 in"); // 50 mm when the field is left out
    assert_eq!(
        named(&commands, "Cabinet Bottom")["corner"][2],
        "Cabinet_plinth"
    );
    assert!(!named(&commands, "Cabinet Side L notch").is_null());

    // legs: no board, the carcass on four legs at the plinth's height, six when wide
    let p = json!({"plinth": 100, "plinth_type": "legs"});
    let (commands, summary) = build(&p).unwrap();
    assert!(named(&commands, "Cabinet Plinth").is_null());
    assert_eq!(count(&commands, "solid.cylinder"), 4);
    let leg = named(&commands, "Cabinet Leg 4");
    assert_eq!(leg["body_name"], "Cabinet Leg 4");
    assert_eq!(leg["height"], "Cabinet_plinth");
    assert_eq!(leg["base"][2], "0 mm");
    assert!(!summary["bodies"].to_string().contains("Leg")); // not a board
    let side = named(&commands, "Cabinet Side L");
    assert_eq!(side["corner"][2], "Cabinet_plinth");
    assert_eq!(side["height"], "Cabinet_height - Cabinet_plinth");
    let wide = json!({"plinth": 100, "plinth_type": "legs", "width": 1000});
    assert_eq!(count(&build(&wide).unwrap().0, "solid.cylinder"), 6);

    // casters: one body each (a plate and a wheel joined), at the legs' places but clear of
    // the sides, four screw holes each in the bottom panel; six when wide
    let p = json!({"plinth": 100, "plinth_type": "casters"});
    let (commands, summary) = build(&p).unwrap();
    assert!(named(&commands, "Cabinet Plinth").is_null());
    let plate = named(&commands, "Cabinet Caster 4");
    assert_eq!(plate["body_name"], "Cabinet Caster 4");
    assert_eq!(plate["height"], "5 mm");
    assert_eq!(plate["corner"][2], "Cabinet_plinth - 5 mm");
    assert_eq!(
        plate["corner"][0],
        "Cabinet_width - (Cabinet_thickness + 50 mm) - 40 mm - Cabinet_width/2"
    );
    let wheel = named(&commands, "Cabinet Caster 4 wheel");
    assert_eq!(wheel["operation"], "join");
    assert_eq!(wheel["targets"], json!(["Cabinet Caster 4"]));
    let wheel_d = "min(75 mm, Cabinet_plinth - 2 mm)"; // stays under the carcass
    assert_eq!(wheel["diameter"], wheel_d);
    assert_eq!(wheel["base"][2], format!("{wheel_d}/2")); // touches the floor
    let post = named(&commands, "Cabinet Caster 4 post");
    assert_eq!(post["operation"], "join");
    assert_eq!(post["targets"], json!(["Cabinet Caster 4"]));
    assert_eq!(post["base"][2], format!("{wheel_d}/2"));
    assert_eq!(
        post["height"],
        format!("Cabinet_plinth - 3 mm - {wheel_d}/2")
    ); // into the plate
    let holes = named(&commands, "Cabinet Bottom caster 1");
    assert_eq!(holes["operation"], "cut");
    assert_eq!(holes["targets"], json!(["Cabinet Bottom"]));
    assert_eq!(holes["base"][2], "Cabinet_plinth");
    assert_eq!(holes["height"], "min(10 mm, Cabinet_thickness/2 - 1 mm)");
    let screws = named(&commands, "Cabinet Bottom caster 1 screws");
    assert_eq!([&screws["count1"], &screws["count2"]], [2, 2]);
    assert_eq!(count(&commands, "solid.pattern.rectangular"), 4);
    assert!(!summary["bodies"].to_string().contains("Caster")); // not a board
    let side = named(&commands, "Cabinet Side R");
    assert_eq!(side["corner"][2], "Cabinet_plinth");
    let wide = json!({"plinth": 100, "plinth_type": "casters", "width": 1000});
    assert_eq!(
        count(&build(&wide).unwrap().0, "solid.pattern.rectangular"),
        6
    );
    let low = json!({"plinth": 40, "plinth_type": "casters"});
    let err = build(&low).unwrap_err();
    assert_eq!(err, "casters need a plinth of at least 50 mm");
    assert!(build(&json!({"plinth": 50, "plinth_type": "casters"})).is_ok());
    let none = json!({"plinth_type": "casters"}); // no plinth: nothing to refuse
    assert_eq!(
        count(&build(&none).unwrap().0, "solid.pattern.rectangular"),
        0
    );

    // no plinth: the type does nothing; a board is the default
    let none = json!({"plinth_type": "legs"});
    assert_eq!(count(&build(&none).unwrap().0, "solid.cylinder"), 0);
    let board = build(&json!({"plinth": 100})).unwrap().0;
    assert!(!named(&board, "Cabinet Plinth").is_null());
    assert_eq!(named(&board, "Cabinet Side L")["corner"][2], "0 mm");
    assert!(build(&json!({"plinth_type": "plank"})).is_err());
}

#[test]
fn the_plinth_set_back_is_a_parameter() {
    let named = |commands: &[(&str, Value)], name: &str| -> Value {
        let found = commands.iter().find(|c| c.1["name"] == name);
        found.map(|c| c.1.clone()).unwrap_or_default()
    };
    let p = json!({"units": "in", "plinth": 4, "plinth_setback": 3});
    let commands = build(&p).unwrap().0;
    assert_eq!(
        named(&commands, "Cabinet_plinth_setback")["expression"],
        "3 in"
    );
    let board = named(&commands, "Cabinet Plinth");
    assert_eq!(
        board["corner"][1],
        "Cabinet_plinth_setback - Cabinet_depth/2"
    );
    // only while a board or notch uses it
    for p in [
        json!({"plinth": 100, "plinth_type": "legs"}),
        json!({"plinth": 0}),
    ] {
        assert!(named(&build(&p).unwrap().0, "Cabinet_plinth_setback").is_null());
    }
    // an edited cabinet that has it keeps it, also on legs
    let p = json!({"plinth": 100, "plinth_type": "legs", "plinth_setback": 80,
                   "_existing": {"plinth_setback": 50.0}});
    let change = named(&build(&p).unwrap().0, "Cabinet_plinth_setback");
    assert_eq!(change["expression"], "80 mm");
    // up to the depth less a panel, and only checked when used
    let deep = json!({"plinth": 100, "plinth_setback": 560 - 18});
    assert!(build(&deep).is_err());
    assert!(build(&json!({"plinth": 100, "plinth_setback": -1})).is_err());
    assert!(build(&json!({"plinth": 100, "plinth_type": "legs", "plinth_setback": 900})).is_ok());
}

#[test]
fn drawers_replace_shelves() {
    let (commands, summary) = build(&json!({"drawers": 3, "shelves": 2, "plinth": 100})).unwrap();
    let named = |n: &str| {
        commands
            .iter()
            .any(|c| c.1["name"] == format!("Cabinet {n}"))
    };
    assert!(named("Drawer 3 Front") && named("Drawer 3 Bottom") && !named("Shelf 1"));
    // 4 carcass, plinth, back, 3 drawers of 6 parts
    assert_eq!(summary["bodies"].as_array().unwrap().len(), 6 + 3 * 6);
    // a pull is two holes across the middle of each front, 128 mm apart
    let (commands, _) = build(&json!({"drawers": 3, "handles": "pull"})).unwrap();
    assert_eq!(count(&commands, "solid.cylinder"), 6 + 3 * 8); // and 8 slide screw holes a drawer
    let hole = commands
        .iter()
        .find(|c| c.1["name"] == "Cabinet Drawer 1 Front handle 2");
    assert_eq!(
        hole.unwrap().1["base"][0],
        "Cabinet_width/2 + 64 mm - Cabinet_width/2"
    );
    // a knob on a wall unit's door: one hole, low on the opening side
    let (commands, _) = build(&json!({"doors": 1, "handles": "knob", "depth": 320})).unwrap();
    let hole = commands
        .iter()
        .find(|c| c.1["name"] == "Cabinet Door handle 1");
    assert_eq!(
        hole.unwrap().1["base"][2],
        "0 mm + 0 mm + Cabinet_reveal + 100 mm + 0 mm"
    );
    assert!(build(&json!({"width": 300, "doors": 2, "handles": "knob"})).is_err());
    // the position can be chosen instead: the middle of a pull, 100 mm + half of 128 from the end
    let height = |at: &str| {
        let p = json!({"doors": 1, "handles": "pull", "handle_position": at});
        let (commands, _) = build(&p).unwrap();
        let hole = commands
            .iter()
            .find(|c| c.1["name"] == "Cabinet Door handle 1");
        hole.unwrap().1["base"][2].as_str().unwrap().to_string()
    };
    assert_eq!(
        height("top"),
        "Cabinet_height - 0 mm - Cabinet_reveal - 164 mm + -64 mm"
    );
    assert_eq!(
        height("middle"),
        "(0 mm + 0 mm + Cabinet_reveal + Cabinet_height - 0 mm - Cabinet_reveal) / 2 + -64 mm"
    );
    assert_eq!(
        height("bottom"),
        "0 mm + 0 mm + Cabinet_reveal + 164 mm + -64 mm"
    );
    assert_eq!(height("auto"), height("top")); // 560 deep: a floor unit
    assert!(build(&json!({"doors": 1, "handles": "knob", "handle_position": "side"})).is_err());
    assert!(build(&json!({"drawers": 2, "doors": 2, "dividers": 1})).is_err());
    assert!(build(&json!({"drawers": 8, "height": 400})).is_err());
}

#[test]
fn dividers_make_equal_compartments() {
    let p = json!({"width": 1200, "dividers": 2, "shelves": 3, "shelf_pins": true});
    let (commands, summary) = build(&p).unwrap();
    // (1200 - 4 panels of 18) / 3
    assert_eq!(summary["inner"]["width"], 376.0);
    let named = |n: &str| {
        commands
            .iter()
            .find(|c| c.1["name"] == format!("Cabinet {n}"))
    };
    assert!(named("Shelf 3.3").is_some());
    assert!(named("Shelf 4.1").is_none());
    // pins: blind in the sides, through the dividers
    assert_eq!(
        named("Divider 1 pins").unwrap().1["height"],
        "Cabinet_thickness"
    );
    assert!(named("Side L pins").unwrap().1["height"]
        .as_str()
        .unwrap()
        .starts_with("min("));
    // 4 carcass, 2 dividers, 9 shelves, back
    assert_eq!(summary["bodies"].as_array().unwrap().len(), 16);
}

#[test]
fn refuses_bad_input() {
    assert!(build(&json!({"width": 300, "dividers": 5})).is_err());
    assert!(build(&json!({"width": 60})).is_err());
    assert!(build(&json!({"height": 200, "shelves": 8})).is_err());
    assert!(build(&json!({"back": "glass"})).is_err());
    assert!(build(&json!({"width": "wide"})).is_err());
    assert!(build(&json!({"units": "cm"})).is_err());
    assert!(build(&json!({"doors": 3})).is_err());
    assert!(build(&json!({"doors": 1, "thickness": 12})).is_err());
    // one door per compartment
    let (commands, _) = build(&json!({"width": 908, "dividers": 2, "doors": 3})).unwrap();
    assert!(commands.iter().any(|c| c.1["name"] == "Cabinet Door 3"));
}

#[test]
fn a_door_hinges_on_its_outer_front_edge() {
    let (commands, summary) = build(&json!({"doors": 2})).unwrap();
    assert_eq!(
        summary["joints"],
        json!(["Cabinet Door L hinge", "Cabinet Door R hinge"])
    );
    let named = |name: &str| &commands.iter().find(|c| c.1["name"] == name).unwrap().1;
    // The hinge line is where two planes meet, in the door and in the cabinet, both by the
    // cabinet's parameters: so it follows a resize. Left door: 2 mm in from the left side.
    // The planes are given outright, so nothing but the axis is added to the design.
    for side in ["", "seat "] {
        let axis = named(&format!("Cabinet Door L hinge {side}axis"));
        let w = "((Cabinet_width - 2*0 mm - 2*Cabinet_reveal - 1*Cabinet_reveal) / 2)";
        let x = format!("0 mm + Cabinet_reveal + ({w} + Cabinet_reveal) * 0 - Cabinet_width/2");
        let y = "-Cabinet_reveal - Cabinet_thickness - Cabinet_depth/2";
        // the order of the planes is the way it opens: this one about -Z
        assert_eq!(
            axis["a"],
            json!({"plane": {"origin": ["0 mm", y, "0 mm"], "normal": [0, 1, 0]}})
        );
        assert_eq!(
            axis["b"],
            json!({"plane": {"origin": [x, "0 mm", "0 mm"], "normal": [1, 0, 0]}})
        );
    }
    // one door hinged on its right instead: the same line at the door's other edge, opening
    // the other way, with the cups there and the handle at the left
    let p = json!({"doors": 1, "door_hinge": "right", "handles": "knob"});
    let (right, _) = build(&p).unwrap();
    let of = |name: &str| &right.iter().find(|c| c.1["name"] == name).unwrap().1;
    let axis = of("Cabinet Door hinge axis");
    assert_eq!(axis["a"]["plane"]["normal"], json!([1, 0, 0]));
    assert_eq!(
        axis["a"]["plane"]["origin"][0],
        "0 mm + Cabinet_reveal + (((Cabinet_width - 2*0 mm - 2*Cabinet_reveal - 0*Cabinet_reveal) / 1) + Cabinet_reveal) * 0 + ((Cabinet_width - 2*0 mm - 2*Cabinet_reveal - 0*Cabinet_reveal) / 1) - Cabinet_width/2"
    );
    assert!(of("Cabinet Door hinge cup 1")["base"][0]
        .as_str()
        .unwrap()
        .contains("- HW_cup_inset"));
    // the plate's screw holes on Side R, into it, about each cup's height
    let plate = of("Cabinet Side R hinge plate 1.2");
    assert_eq!(plate["axis"], json!([1, 0, 0]));
    assert_eq!(plate["targets"], json!(["Cabinet Side R"]));
    assert!(plate["base"][2]
        .as_str()
        .unwrap()
        .ends_with("* HW_pitch + HW_pitch/2"));
    assert!(right
        .iter()
        .all(|c| c.1["targets"] != json!(["Cabinet Side L"])));
    assert!(of("Cabinet Door handle 1")["base"][0]
        .as_str()
        .unwrap()
        .contains("* 0 + 50 mm"));
    // a pair opens from the middle whatever is asked, and three alternate from the first
    let (pair, _) = build(&json!({"doors": 2, "door_hinge": "right"})).unwrap();
    assert_eq!(pair, commands);
    let p = json!({"width": 900, "dividers": 2, "doors": 3, "door_hinge": "right"});
    let (three, _) = build(&p).unwrap();
    let way = |door: usize| {
        let name = format!("Cabinet Door {door} hinge axis");
        let axis = &three
            .iter()
            .find(|c| c.1["name"] == name.as_str())
            .unwrap()
            .1;
        axis["a"]["plane"]["normal"] == json!([1, 0, 0]) // true: hinged on its right
    };
    assert_eq!([way(1), way(2), way(3)], [true, false, true]);
    assert!(build(&json!({"doors": 1, "door_hinge": "top"})).is_err());
    // and the two doors open away from each other
    assert_eq!(
        named("Cabinet Door R hinge axis")["a"]["plane"]["normal"],
        json!([1, 0, 0])
    );
    assert!(commands
        .iter()
        .all(|c| c.0 != "construct.plane.offset" && c.0 != "browser.visibility"));
    let joint = named("Cabinet Door L hinge");
    assert_eq!(
        joint["a"],
        json!({"occurrence": "Cabinet:1", "axis": "Cabinet Door L hinge seat axis"})
    );
    assert_eq!(
        joint["b"],
        json!({"occurrence": "Cabinet Door L:1", "axis": "Cabinet Door L hinge axis"})
    );
    // the axes are folded into the cabinet's history folder
    let group = &commands.last().unwrap().1["features"];
    assert!(group
        .as_array()
        .unwrap()
        .contains(&json!("Cabinet Door L hinge seat axis")));
}

fn named<'a>(commands: &'a [(&str, Value)], n: &str) -> &'a Value {
    let found = (commands.iter()).find(|c| c.1["name"] == n && c.0 != "component.create");
    &found.unwrap_or_else(|| panic!("no {n}")).1
}

#[test]
fn inset_doors_fill_the_opening_less_the_reveal() {
    let p = json!({"fronts": "inset", "doors": 2, "reveal": 3, "handles": "knob"});
    let input = input::read(&Kit::new(&p).unwrap(), &p).unwrap();
    // the shelves behind the doors
    assert_eq!(input.shelf_d, 560.0 - 12.0 - 6.0 - 18.0);
    let (commands, _) = build(&p).unwrap();
    assert_eq!(named(&commands, "Cabinet_reveal")["expression"], "3 mm");
    // between the sides less a reveal at each end and one between: 2 x 279 of 564
    let w = "((Cabinet_width - 2*Cabinet_thickness - 2*Cabinet_reveal - 1*Cabinet_reveal) / 2)";
    let door = named(&commands, "Cabinet Door R");
    assert_eq!(door["length"], w);
    assert_eq!(
        door["corner"],
        json!([
            format!(
                "Cabinet_thickness + Cabinet_reveal + ({w} + Cabinet_reveal) * 1 - Cabinet_width/2"
            ),
            "0 mm - Cabinet_depth/2", // the front face flush with the carcass
            "0 mm + Cabinet_thickness + Cabinet_reveal"
        ])
    );
    // from on the bottom panel to under the top, a reveal off each
    assert_eq!(
        door["height"],
        "Cabinet_height - Cabinet_thickness - Cabinet_reveal - 0 mm - Cabinet_thickness - Cabinet_reveal"
    );
    // cups and handle from the door's back face, at y = thickness; the hinge on its front edge
    for hole in ["Cabinet Door R hinge cup 1", "Cabinet Door R handle 1"] {
        assert_eq!(
            named(&commands, hole)["base"][1],
            "Cabinet_thickness - Cabinet_depth/2"
        );
    }
    let axis = named(&commands, "Cabinet Door L hinge seat axis");
    assert_eq!(axis["a"]["plane"]["origin"][1], "0 mm - Cabinet_depth/2");
    // the shelf starts behind the doors; the sides stay full depth
    let shelf = named(&commands, "Cabinet Shelf 1");
    assert_eq!(shelf["corner"][1], "Cabinet_thickness - Cabinet_depth/2");
    assert_eq!(
        shelf["width"],
        "((Cabinet_depth - 12 mm - Cabinet_back_thickness) - Cabinet_thickness)"
    );
    assert_eq!(named(&commands, "Cabinet Side L")["width"], "Cabinet_depth");
    assert!(build(&json!({"fronts": "flush"})).is_err());
    assert!(build(&json!({"doors": 1, "reveal": 0.2})).is_err());
    assert!(build(&json!({"doors": 1, "reveal": 7})).is_err());
}

#[test]
fn inset_doors_one_per_compartment() {
    // each door fills its own compartment between the dividers, a reveal off each side
    let p = json!({"fronts": "inset", "width": 900, "dividers": 2, "doors": 3});
    let (commands, _) = build(&p).unwrap();
    let cell = "(((Cabinet_width - 2*Cabinet_thickness) - 2*Cabinet_thickness) / 3)";
    let door = named(&commands, "Cabinet Door 2");
    assert_eq!(door["length"], format!("({cell} - 2*Cabinet_reveal)"));
    assert_eq!(
        door["corner"][0],
        format!("Cabinet_thickness + ({cell} + Cabinet_thickness) * 1 + Cabinet_reveal - Cabinet_width/2")
    );
    // the dividers stand behind the doors
    assert_eq!(
        named(&commands, "Cabinet Divider 1")["corner"][1],
        "Cabinet_thickness - Cabinet_depth/2"
    );
}

#[test]
fn overlay_fronts_take_the_reveal() {
    let (commands, _) = build(&json!({"doors": 2, "reveal": 3})).unwrap();
    assert_eq!(named(&commands, "Cabinet_reveal")["expression"], "3 mm");
    let door = named(&commands, "Cabinet Door L");
    assert_eq!(
        door["length"],
        "((Cabinet_width - 2*0 mm - 2*Cabinet_reveal - 1*Cabinet_reveal) / 2)"
    );
    assert_eq!(
        door["corner"][1],
        "-Cabinet_reveal - Cabinet_thickness - Cabinet_depth/2"
    );
    let (commands, _) = build(&json!({"drawers": 1, "reveal": 3})).unwrap();
    assert_eq!(
        named(&commands, "Cabinet Drawer 1 Front")["length"],
        "Cabinet_width - 2*0 mm - 2*Cabinet_reveal"
    );
    // the default, 2 mm, is a parameter too on a cabinet with fronts, and none without
    let (commands, _) = build(&json!({"drawers": 2})).unwrap();
    assert_eq!(named(&commands, "Cabinet_reveal")["expression"], "2 mm");
    let (commands, _) = build(&json!({})).unwrap();
    assert!(commands.iter().all(|c| c.1["name"] != "Cabinet_reveal"));
}

#[test]
fn inset_drawers() {
    // from the bottom panel to the top one, four reveals between three fronts
    let p = json!({"fronts": "inset", "drawers": 3});
    let input = input::read(&Kit::new(&p).unwrap(), &p).unwrap();
    assert_eq!(input.front_h, (720.0 - 36.0 - 8.0) / 3.0);
    let (commands, _) = build(&p).unwrap();
    let front = named(&commands, "Cabinet Drawer 1 Front");
    assert_eq!(
        front["corner"][2],
        "0 mm + Cabinet_thickness + Cabinet_reveal + (((Cabinet_height - 0 mm - 2*Cabinet_thickness - 4*Cabinet_reveal) / 3) + Cabinet_reveal) * 0"
    );
    assert_eq!(
        front["length"],
        "Cabinet_width - 2*Cabinet_thickness - 2*Cabinet_reveal"
    );
    // the box behind the front, clear of the bottom and the top by 8 mm
    let left = named(&commands, "Cabinet Drawer 1 Left");
    assert_eq!(left["corner"][1], "Cabinet_thickness - Cabinet_depth/2");
    assert_eq!(
        left["height"],
        "(((Cabinet_height - 0 mm - 2*Cabinet_thickness - 4*Cabinet_reveal) / 3) - 2*8 mm)"
    );
    // the slide's point on the front's back face, at its lower edge: 18 + 2 up
    assert_eq!(
        named(&commands, "Cabinet Drawer 1 slide")["at"]["point"],
        json!([0.0, -280.0 + 18.0, 20.0])
    );
}

fn has(commands: &[(&str, Value)], n: &str) -> bool {
    commands.iter().any(|c| c.1["name"] == n)
}

#[test]
fn a_sink_base_has_rails_instead_of_a_top() {
    // the dialog's default of one shelf, and pins, are dropped without an error
    let p = json!({"kind": "sink", "doors": 2, "shelf_pins": true, "back": "overlay"});
    let (commands, summary) = build(&p).unwrap();
    assert!(!has(&commands, "Cabinet Top") && !has(&commands, "Cabinet Shelf 1"));
    assert_eq!(count(&commands, "solid.pattern.rectangular"), 0);
    let front = named(&commands, "Cabinet Rail front");
    let back = named(&commands, "Cabinet Rail back");
    for rail in [front, back] {
        assert_eq!(rail["width"], "100 mm");
        assert_eq!(rail["height"], "Cabinet_thickness");
        // the top face at the cabinet's height
        assert_eq!(rail["corner"][2], "Cabinet_height - Cabinet_thickness");
    }
    assert_eq!(front["corner"][1], "0 mm - Cabinet_depth/2");
    // in front of the overlaid back
    assert_eq!(
        back["corner"][1],
        "0 mm + (Cabinet_depth - Cabinet_back_thickness) - 100 mm - Cabinet_depth/2"
    );
    // sides, bottom, two rails, back and two doors
    assert_eq!(summary["bodies"].as_array().unwrap().len(), 8);
    // a grooved back's groove is not cut into a top that is not there
    let (commands, _) = build(&json!({"kind": "sink"})).unwrap();
    let groove = named(&commands, "Cabinet Back groove");
    assert!(!groove["targets"]
        .as_array()
        .unwrap()
        .contains(&json!("Cabinet Top")));
    // inset doors stop under the front rail, as under a top
    let p = json!({"kind": "sink", "doors": 1, "fronts": "inset"});
    let (commands, _) = build(&p).unwrap();
    let door = named(&commands, "Cabinet Door");
    assert_eq!(
        door["height"],
        "Cabinet_height - Cabinet_thickness - Cabinet_reveal - 0 mm - Cabinet_thickness - Cabinet_reveal"
    );
    // dividers (hidden in its dialog, or remembered when Edit makes it a sink base) are
    // dropped; and only these kinds
    let plain = build(&json!({"kind": "sink", "doors": 2})).unwrap().0;
    let p = json!({"kind": "sink", "doors": 2, "dividers": 1, "shelves": 3});
    assert_eq!(build(&p).unwrap().0, plain);
    assert!(build(&json!({"kind": "fridge"})).is_err());
    assert!(build(&json!({"kind": "oven"})).is_err());
}

#[test]
fn a_sink_base_with_drawers_under_a_false_front() {
    let p = json!({"kind": "sink", "drawers": 3, "handles": "knob"});
    let (commands, _) = build(&p).unwrap();
    // the top front is fixed, with no box and no slide, and has its handle hole
    let top = named(&commands, "Cabinet False front");
    assert_eq!(
        top["height"],
        named(&commands, "Cabinet Drawer 2 Front")["height"]
    );
    assert!(!has(&commands, "Cabinet Drawer 3 Front") && !has(&commands, "Cabinet Drawer 3 Left"));
    assert_eq!(count(&commands, "joint.as_built"), 2);
    assert!((commands.iter()).any(|c| c.1["targets"] == json!(["Cabinet False front"])));
    // the boxes below are half the shelves' depth deep, clear of the trap, and slide 4/5 of it
    let side = named(&commands, "Cabinet Drawer 1 Left");
    let half = "((Cabinet_depth - 12 mm - Cabinet_back_thickness)/2)";
    assert_eq!(side["width"], half);
    let slide = commands.iter().find(|c| c.0 == "joint.limits").unwrap();
    assert_eq!(slide.1["max"], (542.0_f64 / 2.0 * 0.8).round());
    // doors or drawers, and two drawers or more
    for p in [
        json!({"kind": "sink", "drawers": 1}),
        json!({"kind": "sink", "drawers": 2, "doors": 2}),
    ] {
        assert!(build(&p).unwrap_err().contains("doors or drawers"));
    }
}

#[test]
fn hidden_fields_left_out_take_their_defaults() {
    // The dialog leaves out a field it hides: no back thickness without a back, even where
    // the 6 mm default would be thicker than the panels; no door or front fields without
    // doors or drawers.
    let thin = json!({"back": "none", "thickness": 5});
    assert!(build(&thin).is_ok());
    let p = json!({"back": "groove", "thickness": 5});
    assert!(build(&p).is_err());
    let full = json!({"doors": 0, "door_hinge": "left", "fronts": "overlay", "reveal": 2,
                      "handles": "none", "handle_position": "auto", "back_thickness": 6});
    assert_eq!(build(&json!({})).unwrap().0, build(&full).unwrap().0);
}

#[test]
fn too_many_commands_for_one_run_are_refused() {
    let p = json!({"width": 2400, "height": 2400, "dividers": 10, "doors": 11, "shelves": 4,
                   "shelf_pins": true, "labels": true, "handles": "pull"});
    let e = build(&p).unwrap_err();
    assert!(e.contains("too many parts"), "{e}");
}

#[test]
fn a_sink_base_too_shallow_for_its_rails_is_refused() {
    // a 6 mm back grooved 12 mm in: 230 - 18 = 212 for the rails, 199 - 18 = 181 is too little
    assert!(build(&json!({"kind": "sink", "depth": 230})).is_ok());
    let e = build(&json!({"kind": "sink", "depth": 199})).unwrap_err();
    assert!(e.contains("rails would overlap"), "{e}");
}

#[test]
fn drawers_over_doors() {
    // a 150 mm drawer front at the top, the doors below: fronts and 3 reveals fill 870 - 150
    let p = json!({"drawers": 1, "doors": 2, "height": 870, "plinth": 150, "shelf_pins": true});
    let input = input::read(&Kit::new(&p).unwrap(), &p).unwrap();
    assert_eq!(
        (input.front_h, input.drawer_h, input.door_h, input.drawer_z),
        (150.0, 150.0, 564.0, 718.0)
    );
    // the shelf stays, below the rail centred behind the reveal at 716 to 718: 708 - 168
    assert_eq!(input.clear_h, 540.0);
    let (commands, summary) = build(&p).unwrap();
    assert_eq!(
        named(&commands, "Cabinet_drawer_height")["expression"],
        "150 mm"
    );
    let z1 = "Cabinet_height - 0 mm - Cabinet_reveal - 1*(Cabinet_drawer_height + Cabinet_reveal)";
    let rail = format!("{z1} + Cabinet_reveal/2 - Cabinet_thickness/2");
    assert_eq!(named(&commands, "Cabinet Rail")["corner"][2], rail);
    let door = named(&commands, "Cabinet Door L");
    assert_eq!(door["corner"][2], "Cabinet_plinth + 0 mm + Cabinet_reveal");
    assert_eq!(
        door["height"],
        format!("{z1} - Cabinet_plinth - 0 mm - Cabinet_reveal")
    );
    let front = named(&commands, "Cabinet Drawer 1 Front");
    assert_eq!(front["height"], "Cabinet_drawer_height");
    assert_eq!(
        front["corner"][2],
        format!("{z1} + Cabinet_reveal + (Cabinet_drawer_height + Cabinet_reveal) * 0")
    );
    // the upper hinge cup about 100 mm under the doors' top edge, on the pins' grid from the
    // bottom panel (the doors are below the rail); the shelf and pins below the rail
    assert_eq!(
        named(&commands, "Cabinet Door L hinge cup 2")["base"][2],
        crate::hardware::on_grid(
            &format!("{z1} - 100 mm"),
            "Cabinet_plinth + Cabinet_thickness"
        )
    );
    let pins = named(&commands, "Cabinet Side L pin rows");
    assert!(pins["count1"].as_str().unwrap().contains(&rail));
    assert_eq!(
        named(&commands, "Cabinet Drawer 1 slide")["at"]["point"][2],
        718.0
    );
    assert_eq!(summary["joints"].as_array().unwrap().len(), 3);
    // one door works too; a door left too low, or a front too low for a box, does not
    assert!(build(&json!({"drawers": 2, "doors": 1})).is_ok());
    assert!(build(&json!({"drawers": 1, "doors": 1, "height": 400})).is_err());
    assert!(build(&json!({"drawers": 1, "doors": 1, "drawer_height": 50})).is_err());
    assert!(build(&json!({"drawers": 1, "doors": 1, "drawers_at": "middle"})).is_err());
}

#[test]
fn doors_over_drawers() {
    // two 140 mm fronts from the plinth up, the door over them: 720 - 280 - 4 reveals
    let p = json!({"drawers": 2, "doors": 1, "drawers_at": "bottom", "height": 870,
                   "plinth": 150, "drawer_height": 140, "shelf_pins": true});
    let input = input::read(&Kit::new(&p).unwrap(), &p).unwrap();
    assert_eq!((input.door_h, input.drawer_z), (432.0, 152.0));
    // between the rail (426 to 444, behind the reveal at 434 to 436) and the top panel
    assert_eq!(input.clear_h, 852.0 - 444.0);
    let (commands, _) = build(&p).unwrap();
    let low = "Cabinet_plinth + 0 mm + Cabinet_reveal";
    let z0 = format!("{low} + 2*(Cabinet_drawer_height + Cabinet_reveal)");
    let rail = format!("{z0} - Cabinet_reveal/2 - Cabinet_thickness/2");
    assert_eq!(named(&commands, "Cabinet Rail")["corner"][2], rail);
    let door = named(&commands, "Cabinet Door");
    assert_eq!(door["corner"][2], z0);
    assert_eq!(
        door["height"],
        "Cabinet_height - 0 mm - Cabinet_reveal - Cabinet_plinth - 0 mm - Cabinet_reveal - 2*(Cabinet_drawer_height + Cabinet_reveal)"
    );
    assert_eq!(
        named(&commands, "Cabinet Drawer 2 Front")["corner"][2],
        format!("{low} + (Cabinet_drawer_height + Cabinet_reveal) * 1")
    );
    assert_eq!(
        named(&commands, "Cabinet Drawer 2 slide")["at"]["point"][2],
        294.0
    );
    // the hinge cups on the pins' grid from the rail's top face
    assert!(named(&commands, "Cabinet Door hinge cup 1")["base"][2]
        .as_str()
        .unwrap()
        .starts_with(&format!("{rail} + Cabinet_thickness + HW_pitch/2 + round(")));
    // the shelf and the pins start on the rail
    let shelf = named(&commands, "Cabinet Shelf 1");
    assert!(shelf["corner"][2]
        .as_str()
        .unwrap()
        .starts_with(&format!("{rail} + Cabinet_thickness + ")));
    assert_eq!(
        named(&commands, "Cabinet Side L pins")["base"][2],
        format!("{rail} + Cabinet_thickness + 64 mm")
    );
}

#[test]
fn inset_drawers_over_a_door() {
    // inside the opening: the door from 20 up to 548, the 150 front from 550 to 700
    let p = json!({"fronts": "inset", "drawers": 1, "doors": 1});
    let input = input::read(&Kit::new(&p).unwrap(), &p).unwrap();
    assert_eq!((input.door_h, input.drawer_z), (528.0, 550.0));
    assert_eq!(input.clear_h, 540.0 - 18.0); // the rail from 540 up
    let (commands, _) = build(&p).unwrap();
    // the rail stands back behind the fronts, as the shelves do
    let rail = named(&commands, "Cabinet Rail");
    assert_eq!(rail["corner"][1], "Cabinet_thickness - Cabinet_depth/2");
    // the box clears the rail by a panel and 8 mm, as an overlay one does
    let z = "Cabinet_height - Cabinet_thickness - Cabinet_reveal - 1*(Cabinet_drawer_height + Cabinet_reveal) + Cabinet_reveal + (Cabinet_drawer_height + Cabinet_reveal) * 0";
    assert_eq!(
        named(&commands, "Cabinet Drawer 1 Left")["corner"][2],
        format!("{z} + (Cabinet_thickness + 8 mm)")
    );
    // in an inch design the height is in inches
    let p = json!({"units": "in", "width": 24, "height": 34.5, "depth": 22, "thickness": 0.75,
                   "drawers": 1, "doors": 2, "drawer_height": 6});
    let (commands, _) = build(&p).unwrap();
    assert_eq!(
        named(&commands, "Cabinet_drawer_height")["expression"],
        "6 in"
    );
}

#[test]
fn a_graduated_drawer_stack() {
    // the top front is given, the other two share what is left of 720 after 4 reveals
    let p = json!({"drawers": 3, "drawer_height": 140});
    let input = input::read(&Kit::new(&p).unwrap(), &p).unwrap();
    assert_eq!((input.front_h, input.drawer_h), (286.0, 140.0));
    let (commands, _) = build(&p).unwrap();
    assert_eq!(
        named(&commands, "Cabinet_drawer_height")["expression"],
        "140 mm"
    );
    let rest = "((Cabinet_height - 0 mm - 2*0 mm - 4*Cabinet_reveal - Cabinet_drawer_height) / 2)";
    assert_eq!(named(&commands, "Cabinet Drawer 1 Front")["height"], rest);
    let top = named(&commands, "Cabinet Drawer 3 Front");
    assert_eq!(top["height"], "Cabinet_drawer_height");
    assert_eq!(
        top["corner"][2],
        format!("0 mm + 0 mm + Cabinet_reveal + ({rest} + Cabinet_reveal) * 2")
    );
    assert_eq!(
        named(&commands, "Cabinet Drawer 3 Left")["height"],
        "(Cabinet_drawer_height - 2*(Cabinet_thickness + 8 mm))"
    );
    assert_eq!(
        named(&commands, "Cabinet Drawer 3 slide")["at"]["point"][2],
        2.0 + 2.0 * 288.0
    );
    assert!(!has(&commands, "Cabinet Rail"));
    // inset: the opening less two panels, (684 - 8 - 140) / 2
    let p = json!({"fronts": "inset", "drawers": 3, "drawer_height": 140});
    let input = input::read(&Kit::new(&p).unwrap(), &p).unwrap();
    assert_eq!(input.front_h, 268.0);
    // 0 is all alike, without a parameter, as one drawer is
    for p in [
        json!({"drawers": 3}),
        json!({"drawers": 1, "drawer_height": 140}),
    ] {
        assert!(!has(&build(&p).unwrap().0, "Cabinet_drawer_height"));
    }
    // edited back to all alike: the parameter goes to 0, so the next edit does not take it up
    let p = json!({"drawers": 3, "drawer_height": 0, "_existing": {"drawer_height": 140.0}});
    let (commands, _) = build(&p).unwrap();
    let change = commands.iter().find(|c| c.0 == "parameters.change");
    assert_eq!(
        change.unwrap().1,
        json!({"name": "Cabinet_drawer_height", "expression": "0 mm"})
    );
    assert!(build(&json!({"drawers": 3, "drawer_height": 600})).is_err());
    assert!(build(&json!({"drawers": 3, "drawer_height": -10})).is_err());
}

#[test]
fn drawer_slides_set_the_box_width_and_drill_both_sides() {
    let (commands, _) = build(&json!({"drawers": 2})).unwrap();
    let box_side = named(&commands, "Cabinet Drawer 1 Left");
    assert_eq!(
        box_side["corner"][0],
        "Cabinet_thickness + HW_slide_t - Cabinet_width/2"
    );
    // into the box side from outside, and into the carcass side beside it, 6 pitches apart
    let hole = named(&commands, "Cabinet Drawer 1 Left slide 2");
    assert_eq!(hole["axis"], json!([1, 0, 0]));
    assert_eq!(hole["diameter"], "HW_box_screw_d");
    assert!(hole["base"][1]
        .as_str()
        .unwrap()
        .contains("HW_setback + 6*HW_pitch"));
    let side = named(&commands, "Cabinet Side R Drawer 2 slide 1");
    assert_eq!(side["axis"], json!([1, 0, 0]));
    assert_eq!(side["targets"], json!(["Cabinet Side R"]));
    assert_eq!(
        side["base"][2],
        hole["base"][2].as_str().unwrap().replace("* 0", "* 1")
    );
    // a sink base's false front has no box and no slide holes
    let (sink, _) = build(&json!({"kind": "sink", "drawers": 2})).unwrap();
    assert_eq!(count(&sink, "solid.cylinder"), 8);
    // 8 drawers still fit in one run, and a box shorter than the slides is refused
    assert!(build(&json!({"drawers": 8, "height": 1400})).is_ok());
    assert!(build(&json!({"drawers": 1, "depth": 260})).is_err());
    assert!(build(&json!({"drawers": 1, "depth": 300})).is_ok());
}

#[test]
fn a_french_cleat_and_its_wall_cleat() {
    let (commands, summary) = build(&json!({"wall_mount": "cleat"})).unwrap();
    let param = |n: &str| named(&commands, n)["expression"].clone();
    assert_eq!(param("Cabinet_cleat_height"), "90 mm");
    assert_eq!(param("Cabinet_cleat_thickness"), "18 mm");
    let (tm, h) = ("Cabinet_cleat_thickness", "Cabinet_cleat_height");
    let cleat = named(&commands, "Cabinet Cleat");
    let wall = named(&commands, "Cabinet Wall cleat");
    let top_z = "Cabinet_height - Cabinet_thickness";
    for (board, z) in [
        (cleat, format!("{top_z} - {h}")),
        (wall, format!("{top_z} - {h} + {tm} - {h}")),
    ] {
        // between the sides, its rear face on theirs, under the top
        assert_eq!(
            [&board["length"], &board["width"], &board["height"]],
            ["(Cabinet_width - 2*Cabinet_thickness)", tm, h]
        );
        assert_eq!(
            board["corner"][1],
            format!("Cabinet_depth - {tm} - Cabinet_depth/2")
        );
        assert_eq!(board["corner"][2], z);
    }
    // bevelled by their thickness: the cabinet's under its front, the wall's on top at the wall
    let bevel = named(&commands, "Cabinet Cleat bevel");
    assert_eq!(
        bevel["edges"],
        json!([{"body": "Cabinet Cleat", "index": 9}])
    );
    assert_eq!(bevel["distance"], tm);
    let bevel = named(&commands, "Cabinet Wall cleat bevel");
    assert_eq!(bevel["edges"][0]["index"], 3);
    // both are boards of the cabinet
    assert!(summary["bodies"].to_string().contains("Cabinet Wall cleat"));
    // the back's groove moves forward by the cleat: shelves follow
    let groove = named(&commands, "Cabinet Back groove");
    assert_eq!(
        groove["corner"][1],
        format!("Cabinet_depth - {tm} - Cabinet_back_thickness - Cabinet_depth/2")
    );
    assert_eq!(
        named(&commands, "Cabinet Shelf 1")["width"],
        format!("(Cabinet_depth - {tm} - Cabinet_back_thickness)")
    );
    // a cleat in inches
    let p = json!({"units": "in", "width": 24, "height": 30, "depth": 12, "thickness": 0.75,
                   "back_thickness": 0.25, "wall_mount": "cleat", "cleat_height": 3.5,
                   "cleat_thickness": 0.75});
    let (commands, _) = build(&p).unwrap();
    assert_eq!(
        named(&commands, "Cabinet_cleat_height")["expression"],
        "3.5 in"
    );
}

#[test]
fn a_hanging_rail() {
    let p = json!({"wall_mount": "rail", "back": "none"});
    let (commands, _) = build(&p).unwrap();
    let rail = named(&commands, "Cabinet Hanging rail");
    assert_eq!(
        [&rail["width"], &rail["height"]],
        ["Cabinet_thickness", "90 mm"]
    );
    assert_eq!(
        rail["corner"][2],
        "Cabinet_height - Cabinet_thickness - 90 mm"
    );
    assert_eq!(count(&commands, "solid.chamfer"), 0);
    assert!(!has(&commands, "Cabinet_cleat_height"));
    // no back: the shelves stop in front of the rail
    assert_eq!(
        named(&commands, "Cabinet Shelf 1")["width"],
        "(Cabinet_depth - Cabinet_thickness)"
    );
    // in a groove: the back right in front of it
    let (commands, _) = build(&json!({"wall_mount": "rail"})).unwrap();
    assert_eq!(
        named(&commands, "Cabinet Back")["corner"][1],
        "Cabinet_depth - Cabinet_thickness - Cabinet_back_thickness - Cabinet_depth/2"
    );
}

#[test]
fn no_wall_mount_is_as_before() {
    let plain = build(&json!({})).unwrap().0;
    assert_eq!(build(&json!({"wall_mount": "none"})).unwrap().0, plain);
    // the groove stays 12 mm from the rear
    assert_eq!(
        named(&plain, "Cabinet Back")["corner"][1],
        "Cabinet_depth - 12 mm - Cabinet_back_thickness - Cabinet_depth/2"
    );
    assert!(build(&json!({"wall_mount": "screws"})).is_err());
}

#[test]
fn a_wall_mount_board_is_refused_behind_an_overlaid_back() {
    for mount in ["cleat", "rail"] {
        let p = json!({"wall_mount": mount, "back": "overlay"});
        let error = build(&p).unwrap_err();
        assert!(error.contains("groove or none"), "{error}");
    }
    // steel needs no board
    assert!(build(&json!({"wall_mount": "steel", "back": "overlay"})).is_ok());
}

#[test]
fn cleats_that_do_not_fit_are_refused() {
    // 2*H - T = 162 mm fits 200 - 36; 2*100 - 18 does not
    let low =
        |h: f64| json!({"wall_mount": "cleat", "height": 200, "shelves": 0, "cleat_height": h});
    assert!(build(&low(90.0)).is_ok());
    assert!(build(&low(100.0)).unwrap_err().contains("do not fit"));
    let p = json!({"wall_mount": "cleat", "cleat_height": 18});
    assert!(build(&p).unwrap_err().contains("taller"));
    let p = json!({"wall_mount": "cleat", "cleat_thickness": 4});
    assert!(build(&p).unwrap_err().contains("6 to 50"));
    let p = json!({"wall_mount": "rail", "height": 120, "shelves": 0});
    assert!(build(&p).unwrap_err().contains("hanging rail"));
}

#[test]
fn a_steel_rail_is_a_parameter_the_cut_list_reads() {
    let (commands, _) = build(&json!({"wall_mount": "steel", "back": "overlay"})).unwrap();
    let rail = named(&commands, "Cabinet_wall_rail");
    assert_eq!(rail["expression"], "Cabinet_width");
    assert!(rail["comment"]
        .as_str()
        .unwrap()
        .contains("steel wall rail"));
    // nothing modelled, the back where it was
    let plain = build(&json!({"back": "overlay"})).unwrap().0;
    assert_eq!(count(&commands, "solid.box"), count(&plain, "solid.box"));
    // an edit away from steel sets it to 0; one that keeps it leaves it alone
    let existing = json!({"width": 600.0, "height": 720.0, "depth": 560.0, "thickness": 18.0,
                          "back_thickness": 6.0, "wall_rail": 600.0});
    let away = build(&json!({"_existing": existing})).unwrap().0;
    let change = named(&away, "Cabinet_wall_rail");
    assert_eq!(change["expression"], "0 mm");
    assert_eq!(count(&away, "parameters.add"), 0);
    let kept = build(&json!({"_existing": existing, "wall_mount": "steel"}))
        .unwrap()
        .0;
    assert!(!has(&kept, "Cabinet_wall_rail"));
    // a cabinet that never had one gets none
    assert!(!has(&plain, "Cabinet_wall_rail"));
}

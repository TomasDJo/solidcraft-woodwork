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
    assert_eq!(count(&commands, "parameters.add"), 6); // with the plinth
    assert_eq!(count(&commands, "solid.box"), 6); // 4 panels, plinth, back

    let p = json!({"shelf_pins": true, "doors": 2, "height": 1200});
    let (commands, summary) = build(&p).unwrap();
    assert_eq!(count(&commands, "solid.pattern.rectangular"), 2);
    assert_eq!(count(&commands, "solid.cylinder"), 2 + 2 * 3); // pin seeds, 3 cups per tall door
    assert_eq!(summary["bodies"].as_array().unwrap().len(), 8);
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
    assert_eq!(count(&commands, "solid.cylinder"), 6);
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
    assert!(build(&json!({"drawers": 2, "doors": 1})).is_err());
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
        .contains("- 22.5 mm"));
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
    // drawers and dividers (hidden in its dialog, or remembered when Edit makes it a sink
    // base) are dropped, also beside its doors; and only these kinds
    let plain = build(&json!({"kind": "sink", "doors": 2})).unwrap().0;
    let p = json!({"kind": "sink", "doors": 2, "drawers": 2, "dividers": 1, "shelves": 3});
    assert_eq!(build(&p).unwrap().0, plain);
    assert!(build(&json!({"kind": "fridge"})).is_err());
    assert!(build(&json!({"kind": "oven"})).is_err());
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

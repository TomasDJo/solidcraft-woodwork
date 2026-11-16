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
    let side = &commands[5].1;
    assert_eq!(side["height"], "Cabinet_height");
    assert_eq!(side["corner"][0], "0 mm - Cabinet_width/2");
    // the history folder holds every feature
    let group = &commands.last().unwrap().1;
    assert_eq!(group["features"].as_array().unwrap().len(), 7);
}

#[test]
fn inches_stay_inches() {
    let p = json!({"units": "in", "width": 24, "height": 30, "depth": 22,
                   "thickness": 0.75, "back_thickness": 0.25, "name": "Base 1"});
    let (commands, summary) = build(&p).unwrap();
    assert_eq!(commands[0].1["name"], "Base_1_width");
    assert_eq!(commands[0].1["expression"], "24 in");
    assert_eq!(summary["inner"]["width"], 24.0 * 25.4 - 2.0 * 19.05);
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
    assert_eq!(hole.unwrap().1["base"][2], "0 mm + 2 mm + 100 mm + 0 mm");
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
    assert_eq!(height("top"), "Cabinet_height - 2 mm - 164 mm + -64 mm");
    assert_eq!(
        height("middle"),
        "(0 mm + 2 mm + Cabinet_height - 2 mm) / 2 + -64 mm"
    );
    assert_eq!(height("bottom"), "0 mm + 2 mm + 164 mm + -64 mm");
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
        let x = "2 mm + (((Cabinet_width - 4 mm - 1*2 mm) / 2) + 2 mm) * 0 - Cabinet_width/2";
        let y = "-2 mm - Cabinet_thickness - Cabinet_depth/2";
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
        "2 mm + (((Cabinet_width - 4 mm - 0*2 mm) / 1) + 2 mm) * 0 + ((Cabinet_width - 4 mm - 0*2 mm) / 1) - Cabinet_width/2"
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

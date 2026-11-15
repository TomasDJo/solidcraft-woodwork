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
    assert_eq!(count(&commands, "solid.hole"), 2 + 2 * 3); // pin seeds, 3 cups per tall door
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
    assert_eq!(count(&commands, "solid.hole"), 6);
    let hole = commands
        .iter()
        .find(|c| c.1["name"] == "Cabinet Drawer 1 Front handle 2");
    assert_eq!(
        hole.unwrap().1["position"][0],
        "Cabinet_width/2 + 64 mm - Cabinet_width/2"
    );
    // a knob on a wall unit's door: one hole, low on the opening side
    let (commands, _) = build(&json!({"doors": 1, "handles": "knob", "depth": 320})).unwrap();
    let hole = commands
        .iter()
        .find(|c| c.1["name"] == "Cabinet Door handle 1");
    assert_eq!(
        hole.unwrap().1["position"][2],
        "0 mm + 2 mm + 100 mm + 0 mm"
    );
    assert!(build(&json!({"width": 300, "doors": 2, "handles": "knob"})).is_err());
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
        named("Divider 1 pins").unwrap().1["depth"],
        "Cabinet_thickness"
    );
    assert!(named("Side L pins").unwrap().1["depth"]
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

use super::*;

fn found() -> select::Cabinet {
    select::Cabinet {
        name: "Base 1".into(),
        min: [0.0, 0.0, 0.0],
        max: [600.0, 554.0, 720.0],
        sides: [[0.0; 4]; 2],
        back: [0.0; 4],
    }
}

#[test]
fn four_joints_of_two_holes_and_a_row() {
    let design = json!({"timeline": [], "params": [
        {"name": "Base_1_depth", "value": 560.0}, {"name": "Base_1_thickness", "value": 18.0},
        {"name": "Base_1_plinth", "value": 100.0},
    ]});
    let commands = build(&found(), &design, "confirmat", "standard").unwrap();
    // sides 554 deep in a 560 cabinet: 3 screws per joint, and an overlaid back
    assert_eq!(commands.len(), 4 * 2 * 3 + 1);
    // each is a cylinder cut out of the one board it is for, not a hole kept by a picked face
    assert_eq!(commands[0].0, "solid.cylinder");
    assert_eq!(commands[0].1["operation"], "cut");
    assert_eq!(commands[0].1["targets"], json!(["Base 1 Side L"]));
    assert_eq!(
        commands[2].1["base"][1],
        "50 mm + ((Base_1_depth - Base_1_back_thickness) - 100 mm) / 2 * 2 - Base_1_depth/2"
    );
    // the screw enters the left side from outside, the bottom sits on the plinth
    assert_eq!(commands[0].1["base"][0], "0 mm - Base_1_width/2");
    assert_eq!(
        commands[0].1["base"][2],
        "Base_1_plinth + Base_1_thickness/2"
    );
    assert_eq!(commands[3].1["targets"][0], "Base 1 Bottom");

    let dowels = build(&found(), &design, "dowel", "standard").unwrap();
    assert_eq!(dowels[0].1["base"][0], "Base_1_thickness - Base_1_width/2");
}

#[test]
fn refuses_twice_and_unknown_cabinets() {
    let params = json!([{"name": "Base_1_depth", "value": 560.0}, {"name": "Base_1_thickness", "value": 18.0}]);
    let drilled =
        json!({"timeline": [{"name": "Base 1 joint Side L Bottom pilot"}], "params": params});
    assert!(build(&found(), &drilled, "dowel", "standard").is_err());
    assert!(build(
        &found(),
        &json!({"timeline": [], "params": []}),
        "dowel",
        "standard"
    )
    .is_err());
    assert!(build(
        &found(),
        &json!({"timeline": [], "params": params}),
        "glue",
        "standard"
    )
    .is_err());
    // a corner cabinet has a `return`
    let corner = json!([{"name": "Base_1_depth", "value": 560.0}, {"name": "Base_1_thickness", "value": 18.0},
        {"name": "Base_1_return", "value": 900.0}]);
    assert!(build(
        &found(),
        &json!({"timeline": [], "params": corner}),
        "dowel",
        "standard"
    )
    .is_err());
}

#[test]
fn a_sink_base_is_screwed_to_its_rails() {
    let design = json!({"timeline": [], "params": [
        {"name": "Base_1_depth", "value": 560.0}, {"name": "Base_1_thickness", "value": 18.0},
    ]});
    let commands = build(&found(), &design, "confirmat", "sink").unwrap();
    // the bottom as usual, 3 a joint; the rails get the first and the last only
    assert_eq!(commands.len(), 2 * 2 * 3 + 2 * 2 * 2 + 1);
    let targets: Vec<&Value> = commands.iter().map(|c| &c.1["targets"][0]).collect();
    assert!(!targets.contains(&&json!("Base 1 Top")));
    let rail = |end: &str| {
        let name = format!("Base 1 joint Side L Rail pilot{end}");
        commands
            .iter()
            .find(|c| c.1["name"] == name.as_str())
            .unwrap()
    };
    assert_eq!(rail("").1["targets"][0], "Base 1 Rail front");
    assert_eq!(rail(" 3").1["targets"][0], "Base 1 Rail back");
}

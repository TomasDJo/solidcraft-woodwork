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
    let (commands, origin) = build(&found(), &design, "confirmat").unwrap();
    assert_eq!(origin, [300.0, 280.0, 0.0]);
    // sides 554 deep in a 560 cabinet: 3 screws per joint, and an overlaid back
    assert_eq!(commands.len(), 4 * 2 * 3 + 1);
    assert_eq!(
        commands[2].1["position"][1],
        "50 mm + ((Base_1_depth - Base_1_back_thickness) - 100 mm) / 2 * 2 - Base_1_depth/2"
    );
    // the screw enters the left side from outside, the bottom sits on the plinth
    assert_eq!(commands[0].1["position"][0], "0 mm - Base_1_width/2");
    assert_eq!(
        commands[0].1["position"][2],
        "Base_1_plinth + Base_1_thickness/2"
    );
    assert_eq!(commands[3].1["targets"][0], "Base 1 Bottom");

    let (dowels, _) = build(&found(), &design, "dowel").unwrap();
    assert_eq!(
        dowels[0].1["position"][0],
        "Base_1_thickness - Base_1_width/2"
    );
}

#[test]
fn refuses_twice_and_unknown_cabinets() {
    let params = json!([{"name": "Base_1_depth", "value": 560.0}, {"name": "Base_1_thickness", "value": 18.0}]);
    let drilled =
        json!({"timeline": [{"name": "Base 1 joint Side L Bottom pilot"}], "params": params});
    assert!(build(&found(), &drilled, "dowel").is_err());
    assert!(build(&found(), &json!({"timeline": [], "params": []}), "dowel").is_err());
    assert!(build(&found(), &json!({"timeline": [], "params": params}), "glue").is_err());
    // a corner cabinet has a `return`
    let corner = json!([{"name": "Base_1_depth", "value": 560.0}, {"name": "Base_1_thickness", "value": 18.0},
        {"name": "Base_1_return", "value": 900.0}]);
    assert!(build(
        &found(),
        &json!({"timeline": [], "params": corner}),
        "dowel"
    )
    .is_err());
}

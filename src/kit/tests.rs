use super::*;

#[test]
fn a_matched_cabinet_points_at_the_selected_ones_parameters() {
    let design = json!({"params": [
        {"name": "Base_1_height", "value": 900.0}, {"name": "Base_1_depth", "value": 400.0},
        {"name": "Base_1_thickness", "value": 19.0}, {"name": "Base_1_width", "value": 800.0},
    ]});
    let link = matched("Base 1", &design).unwrap();
    // the dialog's own height and plinth are ignored: Base 1 has no plinth
    let p = json!({"name": "B", "height": 720, "plinth": 100, "_match": link});
    let (commands, summary) = crate::cabinet::build(&p).unwrap();
    let expression = |name: &str| {
        let found = commands.iter().find(|c| c.1["name"] == name);
        found.map(|c| c.1["expression"].clone())
    };
    assert_eq!(expression("B_height"), Some(json!("Base_1_height")));
    assert_eq!(expression("B_thickness"), Some(json!("Base_1_thickness")));
    assert_eq!(expression("B_width"), Some(json!("600 mm")));
    assert_eq!(expression("B_plinth"), None);
    assert_eq!(summary["size"]["height"], 900.0);
    assert!(matched("Base 2", &design).is_err());
}

#[test]
fn a_taken_name_gets_the_next_free_number() {
    let design = json!({"params": [
        {"name": "Cabinet_width"}, {"name": "Cabinet_2_width"}, {"name": "Base_1_width"},
    ]});
    assert_eq!(free_name("Cabinet", &design), "Cabinet 3");
    assert_eq!(free_name("Base 1", &design), "Base 2");
    assert_eq!(free_name("Base_1", &design), "Base_1 2"); // same parameters as Base 1
    assert_eq!(free_name(" Tall ", &design), "Tall");
}

#[test]
fn a_cabinet_built_again_keeps_its_parameters() {
    // `_existing` has the parameters the design already holds, in mm.
    let p = json!({"name": "A", "units": "in", "_existing": {"width": 609.6, "plinth": 100.0}});
    let kit = Kit::new(&p).unwrap();
    assert_eq!(kit.parameter("width", 609.6, "w"), "A_width"); // as it is: left alone
    kit.parameter("plinth", 127.0, "p"); // another value: changed, not added again
    kit.parameter("depth", 508.0, "d"); // new: added
    let (commands, _) = kit.finish(vec![], [0.0; 3]);
    let made: Vec<_> = commands
        .iter()
        .filter(|c| c.0.starts_with("parameters."))
        .collect();
    assert_eq!(made.len(), 2);
    assert_eq!(made[0].0, "parameters.change");
    assert_eq!(made[0].1, json!({"name": "A_plinth", "expression": "5 in"}));
    assert_eq!(made[1].0, "parameters.add");
    assert_eq!(made[1].1["name"], "A_depth");
}

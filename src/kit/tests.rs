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
fn a_matched_plinth_takes_the_set_back_when_there_is_one() {
    let design = json!({"params": [
        {"name": "A_height", "value": 900.0}, {"name": "A_plinth", "value": 100.0},
        {"name": "A_depth", "value": 560.0}, {"name": "A_thickness", "value": 18.0},
        {"name": "A_plinth_setback", "value": 80.0},
    ]});
    let p = json!({"name": "B", "plinth_setback": 30, "_match": matched("A", &design).unwrap()});
    let expression = |commands: &crate::Commands, name: &str| {
        let found = commands.iter().find(|c| c.1["name"] == name);
        found.map(|c| c.1["expression"].clone())
    };
    let commands = crate::cabinet::build(&p).unwrap().0;
    assert_eq!(
        expression(&commands, "B_plinth_setback"),
        Some(json!("A_plinth_setback"))
    );
    // without one (on legs, or older) the dialog's
    let design = json!({"params": [
        {"name": "A_height", "value": 900.0}, {"name": "A_plinth", "value": 100.0},
        {"name": "A_depth", "value": 560.0}, {"name": "A_thickness", "value": 18.0},
    ]});
    let p = json!({"name": "B", "plinth_setback": 30, "_match": matched("A", &design).unwrap()});
    let commands = crate::cabinet::build(&p).unwrap().0;
    assert_eq!(
        expression(&commands, "B_plinth_setback"),
        Some(json!("30 mm"))
    );
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

#[test]
fn a_board_gets_its_name_written_on_it() {
    let kit = Kit::new(&json!({"name": "A", "labels": true})).unwrap();
    kit.centre("A_width", "A_depth");
    // a side stands in YZ, a top lies in XY, a door stands in XZ; a cut and a board that is
    // not flat along the axes get nothing
    kit.panel(
        "Side L",
        ["0 mm", "0 mm", "0 mm"],
        ["A_thickness", "A_depth", "A_height"],
        None,
    );
    kit.panel(
        "Top",
        ["0 mm", "0 mm", "A_height - A_thickness"],
        ["A_width", "A_depth", "A_thickness"],
        None,
    );
    kit.panel(
        "Door",
        ["2 mm", "-20 mm", "2 mm"],
        ["A_width - 4 mm", "A_thickness", "A_height - 4 mm"],
        None,
    );
    kit.panel(
        "Groove",
        ["0 mm", "0 mm", "0 mm"],
        ["A_width", "A_back_thickness", "A_height"],
        Some(&[]),
    );
    kit.panel(
        "Box",
        ["0 mm", "0 mm", "0 mm"],
        ["A_width", "12 mm", "100 mm"],
        None,
    );
    let (commands, _) = kit.finish(vec![], [0.0; 3]);
    let sketch = |name: &str| &commands.iter().find(|c| c.1["name"] == name).unwrap().1["plane"];
    let labels: Vec<_> = commands.iter().filter(|c| c.0 == "sketch.text").collect();
    assert_eq!(labels.len(), 3);
    assert_eq!(labels[0].1["text"], "A Side L");
    // in the middle of the face, by the board's own expressions, so it stays there
    let side = sketch("A Side L label");
    assert_eq!(
        side["origin"],
        json!([
            "0 mm - A_width/2",
            "0 mm - A_depth/2 + (A_depth) / 2",
            "0 mm + (A_height) / 2"
        ])
    );
    assert_eq!(side["x_dir"], json!([0, -1, 0])); // read from the left
    let top = sketch("A Top label");
    assert_eq!(top["origin"][2], "A_height - A_thickness + A_thickness"); // on its upper face
    assert_eq!(top["y_dir"], json!([0, 1, 0])); // read from above
    let door = sketch("A Door label");
    assert_eq!(door["origin"][1], "-20 mm - A_depth/2"); // on its front face
    assert_eq!(door["y_dir"], json!([0, 0, 1])); // read from the front
                                                 // the sketches sit in the cabinet's history folder, between its boards
    let group = &commands.last().unwrap().1["features"];
    assert!(group.as_array().unwrap().contains(&json!("A Top label")));
    // and without the field there are none
    let plain = Kit::new(&json!({"name": "A"})).unwrap();
    plain.panel(
        "Top",
        ["0 mm"; 3],
        ["A_width", "A_depth", "A_thickness"],
        None,
    );
    assert!(plain
        .finish(vec![], [0.0; 3])
        .0
        .iter()
        .all(|c| c.0 != "sketch.text"));
}

#[test]
fn a_cabinet_prefix_is_never_the_hardwares() {
    assert_eq!(prefix("HW cup"), "c_HW_cup");
    assert_eq!(prefix("HWB"), "HWB");
    assert_eq!(prefix("1 Base"), "c_1_Base");
}

#[test]
fn a_parameter_value_is_found_in_the_reply_text() {
    let reply = r#"{"params": [{"expression": "C_width", "name": "D_width", "unit": "mm", "value": 600.0},
        {"comment": "", "name": "C_width", "unit": "mm", "value": 450.5}, {"name": "C_height", "value": 7.2e2}]}"#;
    assert_eq!(param_value(reply, "C_width"), Some(450.5));
    assert_eq!(param_value(reply, "D_width"), Some(600.0));
    assert_eq!(param_value(reply, "C_height"), Some(720.0));
    assert_eq!(param_value(reply, "C_depth"), None);
}

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

use super::*;

#[test]
fn only_the_missing_parameters_are_made() {
    let all = parameters(&json!({}));
    assert_eq!(all.len(), PARAMETERS.len());
    // every length carries its unit
    let slide = all.iter().find(|c| c.1["name"] == "HW_slide_t").unwrap();
    assert_eq!(slide.1["expression"], "12.7 mm");
    let design = json!({"params": [{"name": "HW_cup_d"}, {"name": "Cabinet_width"}]});
    let some = parameters(&design);
    assert_eq!(some.len(), PARAMETERS.len() - 1);
    assert!(some.iter().all(|c| c.1["name"] != "HW_cup_d"));
}

#[test]
fn a_cabinet_uses_them_with_fronts_only() {
    let built = |p: Value| crate::cabinet::build(&p).unwrap().0;
    assert!(!used(&built(json!({}))));
    assert!(used(&built(json!({"doors": 1}))));
    assert!(used(&built(json!({"drawers": 1}))));
}

#[test]
fn a_cup_goes_where_its_plate_is_on_the_pins_grid() {
    let z = on_grid("Z", "B");
    assert_eq!(
        z,
        "B + HW_pitch/2 + round((Z - (B) - HW_pitch/2) / HW_pitch) * HW_pitch"
    );
    // the same in numbers: base 118 (100 plinth, 18 bottom), cup wanted at 202 (a door from
    // 102, 100 up): 118 + 16 + round(68 / 32) * 32 = 198, plate holes at 182 and 214, pins
    // at 118 + 32k: 182 = 118 + 64 and 214 = 118 + 96
    let snapped = |z: f64, base: f64| base + 16.0 + ((z - base - 16.0) / 32.0).round() * 32.0;
    assert_eq!(snapped(202.0, 118.0), 198.0);
    assert!((snapped(202.0, 118.0) - 202.0).abs() <= 16.0);
}

#[test]
fn a_screw_hole_stops_short_of_a_thin_sides_outside() {
    assert_eq!(screw_depth("T"), "min(HW_screw_depth, T - 3 mm)");
    let (commands, _) = crate::cabinet::build(&json!({"drawers": 1, "thickness": 9})).unwrap();
    let hole = commands
        .iter()
        .find(|c| c.1["name"] == "Cabinet Side L Drawer 1 slide 1");
    assert_eq!(
        hole.unwrap().1["height"],
        "min(HW_screw_depth, Cabinet_thickness - 3 mm)"
    );
    assert!(crate::cabinet::build(&json!({"drawers": 1, "thickness": 5})).is_err());
}

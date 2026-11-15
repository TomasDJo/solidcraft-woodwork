use super::*;
use serde_json::json;

fn named<'a>(commands: &'a Commands, name: &str) -> &'a Value {
    let found = commands.iter().find(|c| c.1["name"] == name);
    &found.unwrap_or_else(|| panic!("no {name}")).1
}

#[test]
fn an_l_of_boards_with_the_corner_cut_out() {
    let p = json!({"width": 1000, "return": 900, "plinth": 100, "doors": "hinged on width"});
    let (commands, summary) = build(&p).unwrap();
    // 2 sides, 2 backs, bottom, top, shelf, 2 plinth boards, 2 doors
    assert_eq!(summary["bodies"].as_array().unwrap().len(), 11);
    assert_eq!(
        summary["size"],
        json!({"width": 1000.0, "height": 720.0, "depth": 900.0})
    );
    assert_eq!(summary["joints"].as_array().unwrap().len(), 2);
    // the return's end is the left side, across the front of the footprint
    assert_eq!(
        named(&commands, "Cabinet Side L")["length"],
        "Cabinet_depth"
    );
    let notch = named(&commands, "Cabinet Shelf 1 notch");
    assert_eq!(notch["operation"], "cut");
    assert_eq!(notch["targets"], json!(["Cabinet Shelf 1"]));
    assert_eq!(notch["corner"][0], "Cabinet_depth - Cabinet_width/2");
}

#[test]
fn a_right_corner_is_the_mirror_image() {
    let p = json!({"corner": "right", "doors": "hinged on width"});
    let (commands, _) = build(&p).unwrap();
    // the return's end is now the right side, and starts `depth` from the right
    assert_eq!(
        named(&commands, "Cabinet Side R")["corner"][0],
        "Cabinet_width - (0 mm) - (Cabinet_depth) - Cabinet_width/2"
    );
    let cup = named(&commands, "Cabinet Door return hinge cup 1");
    assert_eq!(cup["direction"], json!([-1, 0, 0]));
    let joint = commands
        .iter()
        .find(|c| c.1["name"] == "Cabinet Door hinge");
    // hinged at the left end of the back leg: 2 mm in from x = -450
    assert_eq!(joint.unwrap().1["at"]["point"][0], -448.0);
    assert_eq!(joint.unwrap().1["at"]["z"], json!([0, 0, -1]));
}

#[test]
fn a_diagonal_front_is_one_turned_door() {
    let p = json!({"shape": "diagonal", "width": 1000, "return": 900, "plinth": 100,
        "doors": "hinged on width", "handles": "knob"});
    let (commands, summary) = build(&p).unwrap();
    // 2 sides, 2 backs, bottom, top, shelf, plinth, door
    assert_eq!(summary["bodies"].as_array().unwrap().len(), 9);
    assert_eq!(summary["joints"], json!(["Cabinet Door hinge"]));
    // the return's end is as deep as a 45 degree front leaves it
    assert_eq!(
        named(&commands, "Cabinet Side L")["length"],
        "(Cabinet_width - Cabinet_return + Cabinet_depth)"
    );
    // each board's open corner is cut back, picked at mid-thickness
    let cut = named(&commands, "Cabinet Bottom corner");
    assert_eq!(cut["edges"], json!([[482.0, -432.0, 109.0]]));
    assert_eq!(
        cut["distance"],
        "Cabinet_return - Cabinet_depth - 2*Cabinet_thickness"
    );
    let turns: Vec<&Value> = commands
        .iter()
        .filter(|c| c.0 == "occurrence.move")
        .map(|c| &c.1)
        .collect();
    assert_eq!(turns.len(), 2); // plinth and door
    assert_eq!(turns[1]["angle"], "45 deg");
    // hinged at the back leg's end: the far end of the door, on its front edge
    let hinge = named(&commands, "Cabinet Door hinge");
    let at = |i: usize| hinge["at"]["point"][i].as_f64().unwrap();
    assert!((at(0) - 512.728).abs() < 0.01 && (at(1) + 125.557).abs() < 0.01);
    // a return too long for the width leaves no depth at its end
    assert!(build(&json!({"shape": "diagonal", "width": 800, "return": 1300})).is_err());
}

#[test]
fn refuses_bad_input() {
    assert!(build(&json!({"width": 700})).is_err()); // no room beside the return
    assert!(build(&json!({"corner": "middle"})).is_err());
    assert!(build(&json!({"doors": "hinged on width", "thickness": 12})).is_err());
    assert!(build(&json!({"height": 200, "shelves": 8})).is_err());
}

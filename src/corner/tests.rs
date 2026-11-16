use super::*;
use serde_json::json;

fn named<'a>(commands: &'a Commands, name: &str) -> &'a Value {
    let found = commands.iter().find(|c| c.1["name"] == name);
    &found.unwrap_or_else(|| panic!("no {name}")).1
}

/// Where a hinge's line is in what the door hangs on, x and y, and the way it opens about Z:
/// the two planes its axis is made from, and their order.
fn hinge(commands: &Commands, joint: &str) -> (String, String, i32) {
    let axis = named(commands, &format!("{joint} seat axis"));
    let across = |plane: &Value| plane["plane"]["normal"] == json!([1, 0, 0]);
    let (x, y, turn) = match across(&axis["a"]) {
        true => (&axis["a"], &axis["b"], 1),
        false => (&axis["b"], &axis["a"], -1),
    };
    let at = |plane: &Value, i: usize| plane["plane"]["origin"][i].as_str().unwrap().to_string();
    (at(x, 0), at(y, 1), turn)
}

#[test]
fn an_l_of_boards_with_the_corner_cut_out() {
    let p = json!({"width": 1000, "return": 900, "plinth": 100, "doors": "hinged right"});
    let (commands, summary) = build(&p).unwrap();
    // 2 sides, 2 backs, bottom, top, shelf, 2 plinth boards, 2 doors
    assert_eq!(summary["bodies"].as_array().unwrap().len(), 11);
    assert_eq!(
        summary["size"],
        json!({"width": 1000.0, "height": 720.0, "depth": 900.0})
    );
    assert_eq!(summary["joints"].as_array().unwrap().len(), 2);
    // the second door folds out of the cabinet: about the corner where the two front faces
    // meet, turning towards the room
    // (at 1000 x 900, 560 deep: x = 80, y = -130), and it hangs on the first door
    let (x, y, turn) = hinge(&commands, "Cabinet Door return fold");
    assert_eq!(
        x,
        "Cabinet_depth + 2 mm + Cabinet_thickness - Cabinet_width/2"
    );
    assert_eq!(
        y,
        "Cabinet_return - Cabinet_depth - 2 mm - Cabinet_thickness - Cabinet_return/2"
    );
    assert_eq!(turn, 1);
    let fold = named(&commands, "Cabinet Door return fold");
    assert_eq!(fold["a"]["occurrence"], "Cabinet Door:1");
    assert_eq!(fold["b"]["occurrence"], "Cabinet Door return:1");
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
fn the_handle_is_on_the_door_that_folds() {
    let handle_on = |doors: &str, part: &str| {
        let p = json!({"doors": doors, "handles": "pull"});
        let (commands, _) = build(&p).unwrap();
        let wanted = format!("Cabinet {part} handle 1");
        commands.iter().any(|c| c.1["name"] == wanted.as_str())
    };
    // hinged right in a left corner: the back leg's door is on the cabinet, the return's folds
    assert!(handle_on("hinged right", "Door return") && !handle_on("hinged right", "Door"));
    assert!(handle_on("hinged left", "Door") && !handle_on("hinged left", "Door return"));
    // and at the height that is chosen, on both shapes
    for (shape, part) in [("L-shaped", "Door return"), ("diagonal", "Door")] {
        let height = |at: &str| {
            let p = json!({"shape": shape, "doors": "hinged right", "handles": "knob",
                "handle_position": at});
            let (commands, _) = build(&p).unwrap();
            let hole = named(&commands, &format!("Cabinet {part} handle 1"));
            hole["base"][2].as_str().unwrap().to_string()
        };
        assert_eq!(height("top"), "Cabinet_height - 2 mm - 100 mm");
        assert_eq!(
            height("middle"),
            "(0 mm + 2 mm + Cabinet_height - 2 mm) / 2"
        );
        assert_eq!(height("bottom"), "0 mm + 2 mm + 100 mm");
    }
    assert!(build(&json!({"doors": "hinged left", "handle_position": "side"})).is_err());
}

#[test]
fn a_right_corner_is_the_mirror_image() {
    let p = json!({"corner": "right", "doors": "hinged left"});
    let (commands, _) = build(&p).unwrap();
    // the return's end is now the right side, and starts `depth` from the right
    assert_eq!(
        named(&commands, "Cabinet Side R")["corner"][0],
        "Cabinet_width - (0 mm) - (Cabinet_depth) - Cabinet_width/2"
    );
    let cup = named(&commands, "Cabinet Door return hinge cup 1");
    assert_eq!(cup["axis"], json!([-1, 0, 0]));
    // hinged at the left end of the back leg: 2 mm in from the left, x = -448 at 900 wide
    let (x, _, turn) = hinge(&commands, "Cabinet Door hinge");
    assert_eq!(
        x,
        "Cabinet_width - (Cabinet_width - 2 mm) - (0 mm) - Cabinet_width/2"
    );
    assert_eq!(turn, -1);
    // and each joint turns the other way
    assert_eq!(hinge(&commands, "Cabinet Door return fold").2, -1);
}

#[test]
fn a_diagonal_front_is_one_turned_door() {
    let p = json!({"shape": "diagonal", "width": 1000, "return": 900, "plinth": 100,
        "doors": "hinged right", "handles": "knob"});
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
    // (x = 498.586, y = -139.700 at this size), given along the door too, which stands at 45
    let (x, y, turn) = hinge(&commands, "Cabinet Door hinge");
    let along = "(Cabinet_thickness + 4 mm) + (sqrt(2) * (Cabinet_return - Cabinet_depth)) - 2*(Cabinet_thickness + 4 mm)";
    // the last digit of a sine differs between platforms, so it is worked out here as there
    let (sin, cos) = 45f64.to_radians().sin_cos();
    let (sin, cos) = (format!("({sin})"), format!("({cos})"));
    let across = "(-2 mm - Cabinet_thickness)";
    let start = "(Cabinet_width - Cabinet_return + Cabinet_depth) - Cabinet_width/2";
    assert_eq!(x, format!("{start} + ({along}) * {cos} - {across} * {sin}"));
    assert_eq!(
        y,
        format!("0 mm - Cabinet_return/2 + ({along}) * {sin} + {across} * {cos}")
    );
    assert_eq!(turn, 1);
    let joint = named(&commands, "Cabinet Door hinge");
    assert_eq!(joint["angle"], "45 deg");
    // its own line is along the door, and made before the door is turned
    let own = named(&commands, "Cabinet Door hinge axis");
    assert_eq!(own["b"]["plane"]["origin"][1], "-2 mm - Cabinet_thickness");
    let order: Vec<_> = commands.iter().map(|c| (c.0, &c.1["name"])).collect();
    let made = order.iter().position(|c| *c.1 == "Cabinet Door hinge axis");
    let turned = order.iter().rposition(|c| c.0 == "occurrence.move");
    assert!(made < turned);
    // a return too long for the width leaves no depth at its end
    assert!(build(&json!({"shape": "diagonal", "width": 800, "return": 1300})).is_err());
}

#[test]
fn refuses_bad_input() {
    assert!(build(&json!({"width": 700})).is_err()); // no room beside the return
    assert!(build(&json!({"corner": "middle"})).is_err());
    assert!(build(&json!({"doors": "hinged right", "thickness": 12})).is_err());
    assert!(build(&json!({"height": 200, "shelves": 8})).is_err());
}

#[test]
fn the_corner_follows_where_the_cabinet_is_placed() {
    // against a cabinet's right side only a right corner makes sense
    assert_eq!(hand(&json!({"place": "right"})), Ok("right"));
    assert_eq!(
        hand(&json!({"corner": "auto", "place": "right"})),
        Ok("right")
    );
    for place in ["left", "origin", "above"] {
        assert_eq!(hand(&json!({"place": place})), Ok("left"));
    }
    assert_eq!(hand(&json!({})), Ok("left"));
    // and either can be asked for
    assert_eq!(
        hand(&json!({"corner": "left", "place": "right"})),
        Ok("left")
    );
    assert_eq!(hand(&json!({"corner": "right"})), Ok("right"));
    assert!(hand(&json!({"corner": "up"})).is_err());
    // the builder reads it the same way: the return's end is the right side
    let (commands, _) = build(&json!({"place": "right"})).unwrap();
    assert_eq!(
        named(&commands, "Cabinet Side R")["length"],
        "Cabinet_depth"
    );
}

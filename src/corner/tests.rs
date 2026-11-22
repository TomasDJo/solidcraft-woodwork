use super::*;
use serde_json::json;

fn named<'a>(commands: &'a Commands, name: &str) -> &'a Value {
    let found = commands.iter().find(|c| c.1["name"] == name);
    &found.unwrap_or_else(|| panic!("no {name}")).1
}

/// A board's box, not the component of the same name.
fn board<'a>(commands: &'a Commands, name: &str) -> &'a Value {
    let found = commands
        .iter()
        .find(|c| c.0 == "solid.box" && c.1["name"] == name);
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
        "Cabinet_depth + Cabinet_reveal + Cabinet_thickness - Cabinet_width/2"
    );
    assert_eq!(
        y,
        "Cabinet_return - Cabinet_depth - Cabinet_reveal - Cabinet_thickness - Cabinet_return/2"
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
        assert_eq!(height("top"), "Cabinet_height - Cabinet_reveal - 100 mm");
        assert_eq!(
            height("middle"),
            "(0 mm + Cabinet_reveal + Cabinet_height - Cabinet_reveal) / 2"
        );
        assert_eq!(height("bottom"), "0 mm + Cabinet_reveal + 100 mm");
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
        "Cabinet_width - (Cabinet_width - Cabinet_reveal) - (0 mm) - Cabinet_width/2"
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
    let plinth = commands
        .iter()
        .find(|c| c.0 == "solid.box" && c.1["name"] == "Cabinet Plinth");
    assert_eq!(plinth.unwrap().1["corner"][1], "Cabinet_plinth_setback");
    assert_eq!(turns[1]["angle"], "45 deg");
    // hinged at the back leg's end: the far end of the door, on its front edge
    // (x = 498.586, y = -139.700 at this size), given along the door too, which stands at 45
    let (x, y, turn) = hinge(&commands, "Cabinet Door hinge");
    let along = "(Cabinet_thickness + 2*Cabinet_reveal) + (sqrt(2) * (Cabinet_return - Cabinet_depth)) - 2*(Cabinet_thickness + 2*Cabinet_reveal)";
    // the last digit of a sine differs between platforms, so it is worked out here as there
    let (sin, cos) = 45f64.to_radians().sin_cos();
    let (sin, cos) = (format!("({sin})"), format!("({cos})"));
    let across = "(-Cabinet_reveal - Cabinet_thickness)";
    let start = "(Cabinet_width - Cabinet_return + Cabinet_depth) - Cabinet_width/2";
    assert_eq!(x, format!("{start} + ({along}) * {cos} - {across} * {sin}"));
    assert_eq!(
        y,
        format!("0 mm - Cabinet_return/2 + ({along}) * {sin} + {across} * {cos}")
    );
    assert_eq!(turn, 1);
    let joint = named(&commands, "Cabinet Door hinge");
    assert_eq!(joint["angle"], "45 deg");
    // hinged at the other end the axis points down, and the door's turn about it is the other way
    let p = json!({"shape": "diagonal", "doors": "hinged left"});
    let (other, _) = build(&p).unwrap();
    assert_eq!(hinge(&other, "Cabinet Door hinge").2, -1);
    assert_eq!(named(&other, "Cabinet Door hinge")["angle"], "-45 deg");
    // its own line is along the door, and made before the door is turned
    let own = named(&commands, "Cabinet Door hinge axis");
    assert_eq!(
        own["b"]["plane"]["origin"][1],
        "-Cabinet_reveal - Cabinet_thickness"
    );
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
    assert!(build(&json!({"boards": "halves"})).is_err());
    assert!(build(&json!({"boards": "split", "thickness": 15})).is_err());
    // the diagonal's boards stay whole, so no dowels to fit
    assert!(build(&json!({"boards": "split", "thickness": 15, "shape": "diagonal"})).is_ok());
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

#[test]
fn the_doors_take_the_reveal() {
    let (commands, _) = build(&json!({"doors": "hinged right", "reveal": 3})).unwrap();
    assert_eq!(named(&commands, "Cabinet_reveal")["expression"], "3 mm");
    let door = commands
        .iter()
        .find(|c| c.0 == "solid.box" && c.1["name"] == "Cabinet Door");
    assert_eq!(
        door.unwrap().1["height"],
        "Cabinet_height - 0 mm - 2*Cabinet_reveal"
    );
    // no doors, no parameter
    let (commands, _) = build(&json!({})).unwrap();
    assert!(commands.iter().all(|c| c.1["name"] != "Cabinet_reveal"));
    assert!(build(&json!({"fronts": "overlay"})).is_ok());
    assert!(build(&json!({"doors": "hinged right", "fronts": "flush"})).is_err());
    assert!(build(&json!({"doors": "hinged right", "reveal": 9})).is_err());
}

#[test]
fn inset_bifold_doors_fill_the_opening() {
    // hinged right in a left corner: the pair hangs on the back leg's end
    let p = json!({"doors": "hinged right", "fronts": "inset", "handles": "knob", "shelves": 1,
        "shelf_pins": true});
    let (commands, _) = build(&p).unwrap();
    let door = board(&commands, "Cabinet Door");
    // a panel inside the carcass's front, the reveal in from the end panel, bottom and top
    assert_eq!(
        door["corner"][1],
        "Cabinet_return - Cabinet_depth + Cabinet_thickness - Cabinet_thickness - Cabinet_return/2"
    );
    assert_eq!(
        door["height"],
        "Cabinet_height - 0 mm - 2*(Cabinet_thickness + Cabinet_reveal)"
    );
    // it runs into the inner corner, behind the return's door
    assert_eq!(
        door["corner"][0],
        "Cabinet_depth - Cabinet_thickness - Cabinet_width/2"
    );
    // hinged on its front edge, at the carcass's front, the reveal in from the end panel
    let (x, y, turn) = hinge(&commands, "Cabinet Door hinge");
    assert_eq!(
        x,
        "Cabinet_width - (Cabinet_thickness + Cabinet_reveal) - Cabinet_width/2"
    );
    assert_eq!(y, "Cabinet_return - Cabinet_depth - Cabinet_return/2");
    assert_eq!(turn, 1);
    // the second folds about the carcass's inner front corner, and stops the reveal short of
    // the first door's face
    let (x, y, _) = hinge(&commands, "Cabinet Door return fold");
    assert_eq!(x, "Cabinet_depth - Cabinet_width/2");
    assert_eq!(y, "Cabinet_return - Cabinet_depth - Cabinet_return/2");
    let second = board(&commands, "Cabinet Door return");
    assert_eq!(
        second["corner"],
        json!([
            "Cabinet_depth - Cabinet_thickness - Cabinet_width/2",
            "(Cabinet_thickness + Cabinet_reveal) - Cabinet_return/2",
            "0 mm + (Cabinet_thickness + Cabinet_reveal)"
        ])
    );
    assert_eq!(
        second["width"],
        "Cabinet_return - Cabinet_depth + Cabinet_thickness - Cabinet_thickness - Cabinet_reveal - (Cabinet_thickness + Cabinet_reveal)"
    );
    // the handle on the second door, the edge distance in from its free end
    let handle = named(&commands, "Cabinet Door return handle 1");
    assert_eq!(
        handle["base"][1],
        "(Cabinet_thickness + Cabinet_reveal) + 50 mm - Cabinet_return/2"
    );
    // the shelf's notch and pin rows start a panel back, clear of the doors; bottom and top
    // stay full
    let notch = named(&commands, "Cabinet Shelf 1 notch");
    assert_eq!(
        notch["corner"][0],
        "Cabinet_depth - Cabinet_thickness - Cabinet_width/2"
    );
    assert_eq!(notch["width"], "Cabinet_return - Cabinet_depth");
    let bottom = named(&commands, "Cabinet Bottom notch");
    assert_eq!(bottom["corner"][0], "Cabinet_depth - Cabinet_width/2");
    let rows = named(&commands, "Cabinet Back pin rows");
    assert_eq!(
        rows["spacing2"],
        "Cabinet_depth - Cabinet_thickness - Cabinet_thickness - 74 mm"
    );
    // hinged left, on the return: the return's door is the long one, a panel inside the front
    // up to the back leg's door's back face, hinged at the carcass's front by the reveal in
    // from the end; the back leg's door starts the reveal past the return door's face
    let p = json!({"doors": "hinged left", "fronts": "inset"});
    let (commands, _) = build(&p).unwrap();
    let long = board(&commands, "Cabinet Door return");
    assert_eq!(
        long["corner"][0],
        "Cabinet_depth - Cabinet_thickness - Cabinet_width/2"
    );
    assert_eq!(
        long["width"],
        "Cabinet_return - Cabinet_depth + Cabinet_thickness - (Cabinet_thickness + Cabinet_reveal)"
    );
    assert_eq!(
        board(&commands, "Cabinet Door")["corner"][0],
        "Cabinet_depth - Cabinet_thickness + Cabinet_thickness + Cabinet_reveal - Cabinet_width/2"
    );
    let (x, y, turn) = hinge(&commands, "Cabinet Door return hinge");
    assert_eq!(x, "Cabinet_depth - Cabinet_width/2");
    assert_eq!(y, "(Cabinet_thickness + Cabinet_reveal) - Cabinet_return/2");
    assert_eq!(turn, -1);
    let (x, y, _) = hinge(&commands, "Cabinet Door fold");
    assert_eq!(x, "Cabinet_depth - Cabinet_width/2");
    assert_eq!(y, "Cabinet_return - Cabinet_depth - Cabinet_return/2");
    // without doors there is nothing to inset: the shelves stay as they are
    let p = json!({"fronts": "inset"});
    let (commands, _) = build(&p).unwrap();
    let notch = named(&commands, "Cabinet Shelf 1 notch");
    assert_eq!(notch["corner"][0], "Cabinet_depth - Cabinet_width/2");
}

#[test]
fn an_inset_diagonal_door_fits_between_the_ends() {
    let p = json!({"shape": "diagonal", "doors": "hinged right", "fronts": "inset"});
    let (commands, _) = build(&p).unwrap();
    let door = board(&commands, "Cabinet Door");
    // its front face on the front's line, the reveal from the ends' inner corners
    assert_eq!(
        door["corner"],
        json!([
            "(Cabinet_thickness/sqrt(2) + Cabinet_reveal)",
            "0 mm",
            "0 mm + (Cabinet_thickness + Cabinet_reveal)"
        ])
    );
    assert_eq!(
        door["length"],
        "(sqrt(2) * (Cabinet_return - Cabinet_depth)) - 2*(Cabinet_thickness/sqrt(2) + Cabinet_reveal)"
    );
    // hinged on that front face
    let own = named(&commands, "Cabinet Door hinge axis");
    assert_eq!(own["b"]["plane"]["origin"][1], "0 mm");
    // the shelf's slanted edge a panel further back; the bottom's on the front
    let cut = named(&commands, "Cabinet Shelf 1 corner");
    assert_eq!(
        cut["distance"],
        "Cabinet_return - Cabinet_depth - 2*Cabinet_thickness + sqrt(2)*Cabinet_thickness"
    );
    let cut = named(&commands, "Cabinet Bottom corner");
    assert_eq!(
        cut["distance"],
        "Cabinet_return - Cabinet_depth - 2*Cabinet_thickness"
    );
}

#[test]
fn plinth_types() {
    let count = |commands: &Commands, id: &str| commands.iter().filter(|c| c.0 == id).count();
    let has = |commands: &Commands, name: &str| commands.iter().any(|c| c.1["name"] == name);
    // notched: a toe kick in each end's front bottom corner, no plinth boards; mirrored
    let p = json!({"width": 1000, "return": 900, "plinth": 100, "plinth_type": "notch",
                   "corner": "right"});
    let (commands, _) = build(&p).unwrap();
    assert!(!has(&commands, "Cabinet Plinth") && !has(&commands, "Cabinet Plinth return"));
    // a right corner: the return's end is Side R, across the front
    let notch = named(&commands, "Cabinet Side R notch");
    assert_eq!(notch["targets"], json!(["Cabinet Side R"]));
    assert_eq!(
        [&notch["length"], &notch["width"], &notch["height"]],
        [
            "Cabinet_plinth_setback",
            "Cabinet_thickness",
            "Cabinet_plinth"
        ]
    );
    assert_eq!(
        notch["corner"][0],
        "Cabinet_width - (Cabinet_depth - Cabinet_plinth_setback) - (Cabinet_plinth_setback) - Cabinet_width/2"
    );
    let notch = named(&commands, "Cabinet Side L notch");
    assert_eq!(notch["targets"], json!(["Cabinet Side L"]));
    assert_eq!(
        notch["corner"][1],
        "Cabinet_return - Cabinet_depth - Cabinet_return/2"
    );
    // the set-back is the field's: the L's two boards too
    let p = json!({"units": "in", "width": 36, "return": 36, "depth": 22, "plinth": 4,
                   "plinth_setback": 3});
    let (commands, _) = build(&p).unwrap();
    assert_eq!(
        named(&commands, "Cabinet_plinth_setback")["expression"],
        "3 in"
    );
    let plinth = named(&commands, "Cabinet Plinth return");
    assert_eq!(
        plinth["corner"][0],
        "Cabinet_depth - Cabinet_plinth_setback - Cabinet_thickness - Cabinet_width/2"
    );
    assert!(build(&json!({"plinth": 100, "plinth_setback": 560})).is_err());
    // the diagonal's too, without its turned plinth
    let p = json!({"shape": "diagonal", "width": 1000, "return": 900, "plinth": 100,
                   "plinth_type": "notch"});
    let (commands, _) = build(&p).unwrap();
    assert_eq!(count(&commands, "component.create"), 0);
    assert!(has(&commands, "Cabinet Side L notch") && has(&commands, "Cabinet Side R notch"));

    // legs: under the corners of the footprint, six under the L, five under the diagonal;
    // the ends and backs stand on them
    let p = json!({"width": 1000, "return": 900, "plinth": 100, "plinth_type": "legs"});
    let (commands, _) = build(&p).unwrap();
    assert_eq!(count(&commands, "solid.cylinder"), 6);
    assert!(!has(&commands, "Cabinet Plinth"));
    let inner = named(&commands, "Cabinet Leg 6");
    assert_eq!(inner["height"], "Cabinet_plinth");
    assert_eq!(
        inner["base"],
        json!([
            "Cabinet_depth - 50 mm - Cabinet_width/2",
            "Cabinet_return - Cabinet_depth + 50 mm - Cabinet_return/2",
            "0 mm"
        ])
    );
    for part in ["Cabinet Side L", "Cabinet Back", "Cabinet Back return"] {
        let board = board(&commands, part);
        assert_eq!(board["corner"][2], "Cabinet_plinth", "{part}");
        assert_eq!(board["height"], "Cabinet_height - Cabinet_plinth", "{part}");
    }
    let p = json!({"shape": "diagonal", "width": 1000, "return": 900, "plinth": 100,
                   "plinth_type": "legs"});
    let (commands, _) = build(&p).unwrap();
    assert_eq!(count(&commands, "solid.cylinder"), 5);
    assert_eq!(count(&commands, "component.create"), 0);

    // casters are for box cabinets
    let p = json!({"plinth": 100, "plinth_type": "casters"});
    assert_eq!(build(&p).unwrap_err(), "casters are for box cabinets");
}

#[test]
fn split_boards_are_two_rectangles_dowelled_together() {
    let p = json!({"boards": "split", "shelves": 1});
    let (commands, summary) = build(&p).unwrap();
    let bodies = summary["bodies"].to_string();
    for part in ["Bottom", "Top", "Shelf 1"] {
        assert!(
            bodies.contains(&format!("\"Cabinet {part} return\"")),
            "{part}"
        );
        assert!(!commands
            .iter()
            .any(|c| c.1["name"] == format!("Cabinet {part} notch")));
        // two dowels through both boards, across the joint
        for n in [1, 2] {
            let dowel = named(&commands, &format!("Cabinet {part} return dowel {n}"));
            assert_eq!(dowel["axis"], json!([0, 1, 0]));
            let targets = [format!("Cabinet {part}"), format!("Cabinet {part} return")];
            assert_eq!(dowel["targets"], json!(targets));
        }
    }
    // the back leg's board over the whole width, behind the joint
    let bottom = board(&commands, "Cabinet Bottom");
    assert_eq!(bottom["length"], "Cabinet_width - 2*Cabinet_thickness");
    assert_eq!(bottom["width"], "Cabinet_depth - Cabinet_thickness");
    assert_eq!(
        bottom["corner"][1],
        "Cabinet_thickness + Cabinet_return - Cabinet_depth - Cabinet_thickness - Cabinet_return/2"
    );
    // the return's in front of it, as deep as the joint is from the front panel
    let ret = board(&commands, "Cabinet Bottom return");
    assert_eq!(ret["length"], "Cabinet_depth - Cabinet_thickness");
    assert_eq!(
        ret["width"],
        "Cabinet_return - Cabinet_depth - Cabinet_thickness"
    );
    let dowel = named(&commands, "Cabinet Bottom return dowel 2");
    assert_eq!(dowel["base"][0], "Cabinet_depth - 40 mm - Cabinet_width/2");
    assert_eq!(dowel["height"], "60 mm");

    // one piece is the L with its notch, as before
    let (whole, _) = build(&json!({"boards": "one piece"})).unwrap();
    assert_eq!(whole, build(&json!({})).unwrap().0);
    named(&whole, "Cabinet Bottom notch");
    assert!(!whole.iter().any(|c| c.0 == "solid.cylinder"));
    // the diagonal's boards stay whole: split is ignored
    let diagonal = json!({"shape": "diagonal", "boards": "split"});
    let (cut, _) = build(&diagonal).unwrap();
    assert_eq!(cut, build(&json!({"shape": "diagonal"})).unwrap().0);
    // a right corner's dowels are mirrored
    let (right, _) = build(&json!({"boards": "split", "corner": "right"})).unwrap();
    let dowel = named(&right, "Cabinet Top return dowel 1");
    assert_eq!(
        dowel["base"][0],
        "Cabinet_width - (Cabinet_thickness + 40 mm) - (0 mm) - Cabinet_width/2"
    );
}

#[test]
fn split_shelves_behind_inset_doors_start_a_panel_back() {
    let p = json!({"boards": "split", "doors": "hinged right", "fronts": "inset"});
    let (commands, _) = build(&p).unwrap();
    let shelf = board(&commands, "Cabinet Shelf 1 return");
    assert_eq!(
        shelf["length"],
        "Cabinet_depth - Cabinet_thickness - Cabinet_thickness"
    );
    assert_eq!(shelf["width"], "Cabinet_return - Cabinet_depth");
    // the bottom does not
    let bottom = board(&commands, "Cabinet Bottom return");
    assert_eq!(bottom["length"], "Cabinet_depth - Cabinet_thickness");
}

#[test]
fn a_french_cleat_along_both_walls() {
    let (commands, summary) = build(&json!({"wall_mount": "cleat"})).unwrap();
    let (tm, h) = ("Cabinet_cleat_thickness", "Cabinet_cleat_height");
    assert_eq!(named(&commands, h)["expression"], "90 mm");
    let z = format!("Cabinet_height - Cabinet_thickness - {h}");
    // along the back wall, its rear face on the wall, under the top, to the back leg's end
    let cleat = board(&commands, "Cabinet Cleat");
    assert_eq!(
        cleat["corner"][1],
        format!("Cabinet_return - {tm} - Cabinet_return/2")
    );
    assert_eq!(cleat["corner"][2], z);
    assert_eq!(cleat["length"], "Cabinet_width - Cabinet_thickness");
    let wall = board(&commands, "Cabinet Wall cleat");
    assert_eq!(wall["corner"][2], format!("{z} + {tm} - {h}"));
    // along the side wall, from behind the return's end to the back wall's cleat
    let cleat = board(&commands, "Cabinet Cleat return");
    assert_eq!(cleat["corner"][0], "0 mm - Cabinet_width/2");
    assert_eq!(
        [&cleat["length"], &cleat["width"]],
        [
            tm,
            "Cabinet_return - Cabinet_thickness - Cabinet_cleat_thickness"
        ]
    );
    // bevelled under the inner face, and the wall's on top at the wall
    let edge = |n: &str, c: &Commands| {
        named(c, &format!("Cabinet {n} bevel"))["edges"][0]["index"].clone()
    };
    assert_eq!(edge("Cleat", &commands), 9);
    assert_eq!(edge("Wall cleat", &commands), 3);
    assert_eq!(edge("Cleat return", &commands), 11);
    assert_eq!(edge("Wall cleat return", &commands), 0);
    assert!(summary["bodies"]
        .to_string()
        .contains("Cabinet Wall cleat return"));
    // the backs stand off the walls by the cleat, between bottom and top, which reach the walls
    let back = board(&commands, "Cabinet Back");
    assert_eq!(
        back["corner"][1],
        format!("Cabinet_return - Cabinet_thickness - {tm} - Cabinet_return/2")
    );
    assert_eq!(back["corner"][2], "0 mm + Cabinet_thickness");
    let back = board(&commands, "Cabinet Back return");
    assert_eq!(back["corner"][0], format!("{tm} - Cabinet_width/2"));
    let top = board(&commands, "Cabinet Top");
    assert_eq!(top["corner"][0], "0 mm - Cabinet_width/2");
    assert_eq!(top["width"], "Cabinet_return - Cabinet_thickness - 0 mm");
    let shelf = board(&commands, "Cabinet Shelf 1");
    assert_eq!(
        shelf["corner"][0],
        format!("(Cabinet_thickness + {tm}) - Cabinet_width/2")
    );
    // inside the back return and the back leg's end
    assert_eq!(
        shelf["length"],
        format!("Cabinet_width - Cabinet_thickness - (Cabinet_thickness + {tm})")
    );
    // a right corner's bevels are mirrored
    let (right, _) = build(&json!({"wall_mount": "cleat", "corner": "right"})).unwrap();
    assert_eq!(edge("Cleat return", &right), 7);
    assert_eq!(edge("Wall cleat return", &right), 2);
}

#[test]
fn a_hanging_rail_or_steel_rails_on_a_corner() {
    let (commands, _) = build(&json!({"wall_mount": "rail", "shape": "diagonal"})).unwrap();
    let rail = board(&commands, "Cabinet Hanging rail return");
    assert_eq!(
        [&rail["length"], &rail["height"]],
        ["Cabinet_thickness", "90 mm"]
    );
    assert!(!commands
        .iter()
        .any(|c| c.0 == "solid.chamfer" && c.1["name"].as_str().unwrap().contains("bevel")));
    // steel: a rail on each wall, no board
    let (commands, _) = build(&json!({"wall_mount": "steel"})).unwrap();
    assert_eq!(
        named(&commands, "Cabinet_wall_rail")["expression"],
        "Cabinet_width"
    );
    assert_eq!(
        named(&commands, "Cabinet_wall_rail_return")["expression"],
        "Cabinet_return"
    );
    assert_eq!(
        board(&commands, "Cabinet Back")["corner"],
        board(&build(&json!({})).unwrap().0, "Cabinet Back")["corner"]
    );
    // the cleats must fit, and the legs stay deep enough for them
    assert!(build(&json!({"wall_mount": "cleat", "height": 190, "shelves": 0})).is_err());
    assert!(
        build(&json!({"wall_mount": "cleat", "depth": 180, "width": 400, "return": 400})).is_err()
    );
    assert!(build(&json!({"wall_mount": "screws"})).is_err());
}

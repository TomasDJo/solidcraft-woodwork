use super::*;
use serde_json::json;

fn count(commands: &[(&str, Value)], id: &str) -> usize {
    commands.iter().filter(|c| c.0 == id).count()
}

/// The names of the features made, by what they start with after the frame's name.
/// The joints' cuts by method (`dowels`, `bolts`, `bolts nut`): `Frame joint <leg> <rail> ...`.
fn cuts(commands: &[(&str, Value)], method: &str) -> Vec<String> {
    let all = commands.iter().filter(|c| c.0 == "solid.cylinder");
    let all = all.filter_map(|c| c.1["name"].as_str());
    all.filter(|n| n.starts_with("Frame joint ") && n.ends_with(&format!(" {method}")))
        .map(String::from)
        .collect()
}

fn frame(p: Value) -> (Commands, Value) {
    let mut p = p;
    p["name"] = json!("Frame");
    build(&p).unwrap()
}

#[test]
fn default_frame_four_legs_aprons_and_end_stretchers() {
    let (commands, summary) = frame(json!({}));
    let bodies: Vec<&str> = (summary["bodies"].as_array().unwrap().iter())
        .filter_map(Value::as_str)
        .collect();
    assert_eq!(
        bodies,
        [
            "Frame Leg front left",
            "Frame Leg front right",
            "Frame Leg back left",
            "Frame Leg back right",
            "Frame Apron L",
            "Frame Apron R",
            "Frame Apron front",
            "Frame Apron back",
            "Frame Stretcher L",
            "Frame Stretcher R",
        ]
    );
    // none of them a cabinet's leg (hardware); all solid timber for the cut list
    assert!(bodies
        .iter()
        .all(|b| !crate::fittings::is_leg(b) && is_timber(b)));
    // eight parameters, every size an expression over them; the dowel's two and its count
    assert_eq!(count(&commands, "parameters.add"), 8 + 3);
    assert_eq!(commands[0].1["name"], "Frame_width");
    let box_of = |name: &str| {
        let found = commands
            .iter()
            .find(|c| c.0 == "solid.box" && c.1["name"] == name);
        found.unwrap().1.clone()
    };
    let front = box_of("Frame Apron front");
    assert_eq!(front["length"], "Frame_width - 2*Frame_leg");
    assert_eq!(front["width"], "Frame_apron_thickness");
    assert_eq!(front["height"], "Frame_apron_height");
    assert_eq!(
        front["corner"],
        json!([
            "Frame_leg - Frame_width/2",
            "Frame_inset - Frame_depth/2",
            "Frame_height - Frame_apron_height"
        ])
    );
    assert_eq!(
        box_of("Frame Stretcher L")["corner"][2],
        "Frame_stretcher_height"
    );
    assert_eq!(box_of("Frame Leg back right")["height"], "Frame_height");
    // two dowels at each end of each of the six rails, each through leg and rail: one cut at
    // each end and one pattern of two for all, a quarter of the rail's height in from each edge
    let dowels = cuts(&commands, "dowels");
    assert_eq!(dowels.len(), 12);
    let first = commands
        .iter()
        .find(|c| c.1["name"] == "Frame joint Leg front left Apron L dowels")
        .unwrap();
    assert_eq!(
        first.1["targets"],
        json!(["Frame Leg front left", "Frame Apron L"])
    );
    assert_eq!(first.1["operation"], "cut");
    assert_eq!(first.1["diameter"], "Frame_dowel_d");
    assert_eq!(first.1["height"], "Frame_dowel_length + 2 mm");
    let z = first.1["base"][2].as_str().unwrap();
    assert_eq!(
        z,
        "Frame_height - Frame_apron_height + (Frame_apron_height/4)"
    );
    let rows: Vec<_> = (commands.iter())
        .filter(|c| c.0 == "solid.pattern.rectangular")
        .collect();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].1["features"].as_array().unwrap().len(), 12);
    assert_eq!(rows[0].1["dir1"], json!([0, 0, 1]));
    // seats at the back corners, so a cabinet placed beside it links to it; one folder
    assert_eq!(count(&commands, "construct.axis.two_planes"), 2);
    assert_eq!(commands.last().unwrap().0, "timeline.group");
    assert_eq!(
        summary["size"],
        json!({"width": 1200.0, "height": 870.0, "depth": 600.0})
    );
}

#[test]
fn six_legs_split_the_long_rails_and_add_middle_ones() {
    let (commands, summary) = frame(json!({"legs": "6", "stretchers": "all"}));
    let bodies = summary["bodies"].as_array().unwrap();
    assert_eq!(bodies.len(), 6 + 7 + 7);
    for part in [
        "Leg middle front",
        "Leg middle back",
        "Apron middle",
        "Apron front L",
        "Apron back R",
        "Stretcher middle",
        "Stretcher front R",
    ] {
        assert!(bodies.contains(&json!(format!("Frame {part}"))), "{part}");
    }
    assert!(!bodies.contains(&json!("Frame Apron front")));
    assert_eq!(cuts(&commands, "dowels").len(), 14 * 2);
    // well inside a run's 256 commands
    assert!(commands.len() < 150, "{}", commands.len());
}

#[test]
fn no_stretchers_no_parameter() {
    let (commands, summary) = frame(json!({"stretchers": "none", "stretcher_height": 9999}));
    assert_eq!(summary["bodies"].as_array().unwrap().len(), 8);
    assert_eq!(count(&commands, "parameters.add"), 7 + 3);
}

#[test]
fn bolts_go_through_the_outer_faces_and_dowels_join_at_the_middle_legs() {
    let (commands, _) = frame(json!({"joint": "bolts"}));
    // a bolt and a barrel nut at each end of the six rails
    assert_eq!(cuts(&commands, "bolts").len(), 12);
    assert_eq!(cuts(&commands, "bolts nut").len(), 12);
    assert!(cuts(&commands, "dowels").is_empty());
    let bolt = &commands
        .iter()
        .find(|c| c.1["name"] == "Frame joint Leg front left Apron L bolts")
        .unwrap()
        .1;
    // Apron L, from the front face of the front left leg, along +Y, at a third of its height
    assert_eq!(bolt["axis"], json!([0, 1, 0]));
    assert_eq!(bolt["height"], "Frame_leg + 80 mm");
    assert_eq!(
        bolt["base"],
        json!([
            "Frame_inset + Frame_apron_thickness/2 - Frame_width/2",
            "Frame_leg - (Frame_leg) - Frame_depth/2",
            "Frame_height - Frame_apron_height + Frame_apron_height/3 + (0 mm / 2)"
        ])
    );
    let nut = &commands
        .iter()
        .find(|c| c.1["name"] == "Frame joint Leg front left Apron L bolts nut")
        .unwrap()
        .1;
    assert_eq!(nut["axis"], json!([-1, 0, 0])); // from the inside face
    assert_eq!(nut["targets"], json!(["Frame Apron L"]));
    // the front apron's bolt is higher, so the two in a corner leg do not meet
    let front = (commands.iter())
        .find(|c| c.1["targets"] == json!(["Frame Leg front left", "Frame Apron front"]))
        .unwrap();
    assert!(front.1["base"][2].as_str().unwrap().contains("*2/3"));
    // with six legs the split rails meet the middle legs with dowels
    let (commands, _) = frame(json!({"joint": "bolts", "legs": "6", "stretchers": "none"}));
    assert_eq!(cuts(&commands, "dowels").len(), 4);
    assert_eq!(cuts(&commands, "bolts").len(), 7 * 2 - 4);
}

#[test]
fn inches_carry_their_unit() {
    let p = json!({"units": "in", "width": 48, "depth": 24, "height": 34, "leg": 2.75,
        "apron_height": 4, "apron_thickness": 0.875, "inset": 0.375, "stretcher_height": 6});
    let (commands, summary) = frame(p);
    assert_eq!(commands[0].1["expression"], "48 in");
    let inset = commands
        .iter()
        .find(|c| c.1["name"] == "Frame_inset")
        .unwrap();
    assert_eq!(inset.1["expression"], "0.375 in");
    assert_eq!(summary["size"]["width"], 48.0 * 25.4);
}

#[test]
fn bad_input_names_the_field() {
    let refused = |p: Value| build(&p).unwrap_err();
    assert!(refused(json!({"leg": 10})).contains("`leg`"));
    assert!(refused(json!({"width": 230})).contains("`width`"));
    // six legs need room for two spans
    assert!(refused(json!({"width": 400, "legs": "6"})).contains("`width`"));
    assert!(build(&json!({"width": 400})).is_ok());
    assert!(refused(json!({"depth": 200})).contains("`depth`"));
    assert!(refused(json!({"apron_height": 900})).contains("`apron_height`"));
    assert!(refused(json!({"apron_thickness": 8})).contains("`apron_thickness`"));
    assert!(refused(json!({"inset": 50, "apron_thickness": 22})).contains("`inset`"));
    assert!(refused(json!({"inset": -1})).contains("`inset`"));
    assert!(refused(json!({"stretcher_height": 700})).contains("`stretcher_height`"));
    assert!(refused(json!({"stretchers": "some"})).contains("`stretchers`"));
    assert!(refused(json!({"legs": "5"})).contains("`legs`"));
    assert!(refused(json!({"joint": "glue"})).contains("`joint`"));
    assert!(refused(json!({"place": "above"})).contains("`place`"));
}

#[test]
fn timber_is_a_frames_part_not_a_cabinets_leg() {
    assert!(is_timber("Bench Leg middle back"));
    assert!(is_timber("Bench Stretcher front L"));
    assert!(!is_timber("Base Leg 3"));
    assert!(!is_timber("Base Side L"));
}

#[test]
fn dowels_and_bolts_that_would_meet_are_refused() {
    let refused = |p: Value| build(&p).unwrap_err();
    // six legs of 40 mm: the split rails' dowels meet in the middle leg
    let err = refused(json!({"legs": "6", "leg": 40, "inset": 0, "apron_thickness": 12}));
    assert!(err.contains("`leg`") && err.contains("6 legs"), "{err}");
    assert!(build(&json!({"legs": "6", "leg": 50, "inset": 0, "apron_thickness": 12})).is_ok());
    // at a corner the two rails' dowels cross: 10 + 11 + 24 = 45 mm
    let err = refused(json!({"leg": 45}));
    assert!(err.contains("`leg`") && err.contains("corner"), "{err}");
    assert!(build(&json!({"leg": 45, "joint": "bolts"})).is_ok());
    // bolts from both ends of a short rail would meet: 159 mm between the legs
    let err = refused(json!({"joint": "bolts", "depth": 299}));
    assert!(err.contains("`depth`"), "{err}");
    assert!(build(&json!({"joint": "bolts", "depth": 300})).is_ok());
    let err = refused(json!({"joint": "bolts", "width": 299}));
    assert!(err.contains("`width`"), "{err}");
    // with six legs the long rails are split, bolted at one end only
    assert!(build(&json!({"joint": "bolts", "legs": "6", "width": 420})).is_ok());
}

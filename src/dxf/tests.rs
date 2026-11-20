use super::*;

/// A board 600 along x, 400 along y, 18 thick, as `model.faces` gives it, with these round faces.
fn board(holes: &[Value]) -> Value {
    let flat = |n: [f64; 3], c: [f64; 3], area: f64| json!({"plane_normal": n, "centroid": c, "area": area, "cylinder": null});
    let mut faces = vec![
        flat([0.0, 0.0, -1.0], [300.0, 200.0, 0.0], 240000.0),
        flat([0.0, 0.0, 1.0], [300.0, 200.0, 18.0], 240000.0),
        flat([-1.0, 0.0, 0.0], [0.0, 200.0, 9.0], 7200.0),
        flat([1.0, 0.0, 0.0], [600.0, 200.0, 9.0], 7200.0),
        flat([0.0, -1.0, 0.0], [300.0, 0.0, 9.0], 10800.0),
        flat([0.0, 1.0, 0.0], [300.0, 400.0, 9.0], 10800.0),
    ];
    faces.extend(holes.iter().cloned());
    json!({"faces": faces})
}

fn hole(origin: [f64; 3], axis: [f64; 3], r: f64, centroid: [f64; 3]) -> Value {
    json!({"plane_normal": null, "centroid": centroid, "area": 1.0,
           "cylinder": {"origin": origin, "axis": axis, "radius": r}})
}

/// A 35 mm cup 13 deep in the top at (100, 50), and the same face again (cut in two).
fn cup() -> Value {
    hole(
        [100.0, 50.0, 18.0],
        [0.0, 0.0, -1.0],
        17.5,
        [100.0, 50.0, 11.5],
    )
}

#[test]
fn holes_land_on_the_part_as_it_lies_on_the_sheet() {
    let pin = hole(
        [500.0, 300.0, 0.0],
        [0.0, 0.0, 1.0],
        2.5,
        [500.0, 300.0, 6.0],
    );
    let through = hole(
        [300.0, 200.0, 0.0],
        [0.0, 0.0, 1.0],
        4.0,
        [300.0, 200.0, 9.0],
    );
    let edge = hole([0.0, 200.0, 9.0], [1.0, 0.0, 0.0], 3.5, [10.0, 200.0, 9.0]);
    let faces = board(&[cup(), cup(), pin, through, edge]);
    // Lying flat, its length along the sheet. Seen from below (it may lie either face up),
    // so x is mirrored: the cup is at the far end and drilled from the back.
    let flat = json!({"x": 10, "y": 10, "w": 600, "h": 400, "rotated": false});
    let (drawn, into_edge) = holes(&faces, &flat);
    assert_eq!(into_edge, 1);
    assert_eq!(
        drawn,
        [
            json!([510.0, 60.0, 35.0, 13.0, true]),
            json!([110.0, 310.0, 5.0, 12.0, false]),
            json!([310.0, 210.0, 8.0, null, false]),
        ]
    );
    // Turned a quarter, its length up the sheet: the same picture turned, not mirrored.
    let turned = json!({"x": 10, "y": 10, "w": 400, "h": 600, "rotated": true});
    let (drawn, _) = holes(&faces, &turned);
    assert_eq!(drawn[0], json!([360.0, 510.0, 35.0, 13.0, true]));
    assert_eq!(drawn[1], json!([110.0, 110.0, 5.0, 12.0, false]));
}

#[test]
fn a_board_is_seen_from_the_face_with_most_holes() {
    // The cup alone: the board is drawn from the cup's side, mirrored the other way.
    let flat = json!({"x": 10, "y": 10, "w": 600, "h": 400, "rotated": false});
    let (drawn, _) = holes(&board(&[cup()]), &flat);
    assert_eq!(drawn, [json!([510.0, 360.0, 35.0, 13.0, false])]);
    // No flat faces: nothing to go by.
    assert_eq!(holes(&json!({}), &flat), (vec![], 0));
}

#[test]
fn hole_layers() {
    assert_eq!(layer(5.0, Some(12.0), false), "DRILL_5_D12");
    assert_eq!(layer(35.000001, Some(12.96), true), "DRILL_35_D13_BACK");
    assert_eq!(layer(7.0, None, false), "DRILL_7_THRU");
    assert_eq!(layer(4.5, Some(9.04), false), "DRILL_4P5_D9");
    assert_eq!(layer(5.0, Some(10.92), false), "DRILL_5_D10P9");
}

#[test]
fn a_sheet_as_dxf_r12() {
    let layout = json!({"material": "Oak", "thickness": 18.0, "size": [2440, 1220], "parts": [
        {"part": "Cabinet Door", "x": 10, "y": 10, "w": 400, "h": 700, "rotated": false,
         "holes": [[32.5, 100, 35, 13, false], [32.5, 600, 35, 13, false], [370, 350, 5, null, true]]},
        {"part": "Cabinet Shelf 1", "x": 420, "y": 10, "w": 564, "h": 300, "rotated": false},
    ]});
    let dxf = render(&layout, 10.0);
    let lines: Vec<&str> = dxf.lines().collect();
    // group code and value on alternate lines, codes are numbers
    assert_eq!(lines.len() % 2, 0);
    assert!(lines.iter().step_by(2).all(|c| c.parse::<u32>().is_ok()));
    let pairs: Vec<(&str, &str)> = lines.chunks(2).map(|p| (p[0], p[1])).collect();
    let at = |pair: (&str, &str)| pairs.iter().position(|p| *p == pair).unwrap();
    assert_eq!(
        pairs[..4],
        [
            ("0", "SECTION"),
            ("2", "HEADER"),
            ("9", "$ACADVER"),
            ("1", "AC1009")
        ]
    );
    assert_eq!(pairs[pairs.len() - 1], ("0", "EOF"));
    assert!(at(("9", "$INSUNITS")) < at(("2", "TABLES")));
    assert!(at(("2", "TABLES")) < at(("2", "ENTITIES")));
    // every layer an entity is on is in the layer table, and only those
    let table: Vec<&str> = (pairs.windows(2))
        .filter(|w| w[0] == ("0", "LAYER"))
        .map(|w| w[1].1)
        .collect();
    let used: std::collections::BTreeSet<&str> = (pairs[at(("2", "ENTITIES"))..].iter())
        .filter(|p| p.0 == "8")
        .map(|p| p.1)
        .collect();
    assert_eq!(
        table,
        [
            "SHEET",
            "OUTLINE",
            "TEXT",
            "DRILL_35_D13",
            "DRILL_5_THRU_BACK"
        ]
    );
    assert_eq!(used, table.iter().copied().collect());
    let count = |what: &str| pairs.iter().filter(|p| *p == &("0", what)).count();
    // sheet and trim, two parts; a name each; three holes
    assert_eq!(
        (count("POLYLINE"), count("VERTEX"), count("SEQEND")),
        (4, 16, 4)
    );
    assert_eq!((count("TEXT"), count("CIRCLE")), (2, 3));
    assert!(dxf.contains("10\n2430\n20\n1210\n")); // the trim's far corner
    assert!(dxf.contains("1\nCabinet Door\n50\n90\n")); // up the tall door
    assert!(dxf.contains("40\n17.5\n")); // a cup's radius
                                         // R12 layers name their line type, from an LTYPE table before the LAYER table
    assert!(at(("2", "CONTINUOUS")) < at(("2", "LAYER")));
    assert_eq!(dxf.matches("\n6\nCONTINUOUS\n").count(), table.len());
    // A name stays inside its part: no longer than the part's long side, with characters
    // as wide as they are high at most.
    let starts = (0..pairs.len()).filter(|&i| pairs[i] == ("0", "TEXT"));
    let field = |at: usize, code: &str| pairs[at..].iter().find(|p| p.0 == code).unwrap().1;
    let names: Vec<(f64, usize)> = starts
        .map(|i| {
            (
                field(i, "40").parse().unwrap(),
                field(i, "1").chars().count(),
            )
        })
        .collect();
    assert_eq!(names.len(), 2);
    for ((height, chars), long) in names.into_iter().zip([700.0, 564.0]) {
        assert!(height * chars as f64 <= long, "{height} x {chars} > {long}");
    }
    // no trim, no parts: the sheet alone
    let empty = render(&json!({"size": [2440, 1220], "parts": []}), 0.0);
    assert_eq!(empty.matches("POLYLINE").count(), 1);
    assert!(!empty.contains("OUTLINE"));
}

#[test]
fn a_file_per_sheet() {
    let sheet =
        |material: &str, thickness: f64| json!({"material": material, "thickness": thickness});
    let layouts = [
        sheet("White melamine", 18.0),
        sheet("White melamine", 18.0),
        sheet("Oak/veneer", 18.0),
        sheet("White melamine", 12.5),
    ];
    assert_eq!(
        files("kitchen", &layouts),
        [
            "kitchen-White melamine-18-1.dxf",
            "kitchen-White melamine-18-2.dxf",
            "kitchen-Oakveneer-18-1.dxf",
            "kitchen-White melamine-12.5-1.dxf",
        ]
    );
}

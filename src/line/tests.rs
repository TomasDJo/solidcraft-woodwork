use super::*;
use serde_json::json;

/// `sketch.inspect`'s reply for one line from `a` to `b` on the floor, with a length dimension
/// `k1` (driven or not) and other points.
fn sketch(a: [f64; 2], b: [f64; 2], driven: bool, others: &[[f64; 2]]) -> Value {
    let mut points = vec![
        json!({"id": "origin", "world": [0.0, 0.0, 0.0]}),
        json!({"id": "l1.start", "world": [a[0], a[1], 0.0]}),
        json!({"id": "l1.end", "world": [b[0], b[1], 0.0]}),
    ];
    for (i, p) in others.iter().enumerate() {
        points.push(json!({"id": format!("p{i}"), "world": [p[0], p[1], 0.0]}));
    }
    json!({
        "curves": [{"id": "c9", "type": "circle"}, {"id": "l1", "type": "line", "start": "l1.start", "end": "l1.end"}],
        "points": points,
        "constraints": [{"id": "k1", "type": "length", "l": 1, "param": "d1", "driven": driven}],
    })
}

#[test]
fn a_sketch_line_along_x_faces_minus_y_and_names_its_dimension() {
    let line = from_sketch(&sketch([200.0, 500.0], [1400.0, 500.0], false, &[]), "l1").unwrap();
    assert_eq!(line.front, [0.0, -1.0]);
    assert_eq!(line.dimension.as_deref(), Some("d1"));
    assert_eq!(length(&line), 1200.0);
    // a driven dimension has no parameter to follow
    let driven = from_sketch(&sketch([200.0, 500.0], [1400.0, 500.0], true, &[]), "l1").unwrap();
    assert_eq!(driven.dimension, None);
}

#[test]
fn a_sketch_line_faces_the_rest_of_its_sketch() {
    // a room drawn above the line: the cabinet faces into it
    let room = from_sketch(
        &sketch(
            [0.0, 0.0],
            [3000.0, 0.0],
            false,
            &[[0.0, 2000.0], [3000.0, 2000.0]],
        ),
        "l1",
    )
    .unwrap();
    assert_eq!(room.front, [0.0, 1.0]);
    let wall = from_sketch(
        &sketch([0.0, 0.0], [0.0, 2000.0], false, &[[3000.0, 0.0]]),
        "l1",
    )
    .unwrap();
    assert_eq!(wall.front, [1.0, 0.0]);
}

#[test]
fn only_level_lines_along_x_or_y() {
    let err = from_sketch(&sketch([0.0, 0.0], [1000.0, 577.35], false, &[]), "l1").unwrap_err();
    assert!(
        err.contains("along X or Y") && err.contains("30.0 degrees"),
        "{err}"
    );
    let mut sloped = sketch([0.0, 0.0], [1000.0, 0.0], false, &[]);
    sloped["points"][2]["world"] = json!([1000.0, 0.0, 50.0]);
    assert!(from_sketch(&sloped, "l1").unwrap_err().contains("level"));
    let mut circle = sketch([0.0, 0.0], [1000.0, 0.0], false, &[]);
    circle["curves"][1]["type"] = json!("arc");
    assert!(from_sketch(&circle, "l1").unwrap_err().contains("straight"));
}

/// `model.edges` and `model.faces` of a 2000 x 100 x 2400 wall from (0, 500, 0).
fn wall() -> (Value, Value) {
    let edges = json!({"edges": [
        {"index": 1, "name": "F1:+z|F1:-y", "start": [0.0, 500.0, 2400.0], "end": [2000.0, 500.0, 2400.0], "length": 2000.0},
        {"index": 5, "name": "F1:-y|F1:-z", "start": [2000.0, 500.0, 0.0], "end": [0.0, 500.0, 0.0], "length": 2000.0},
        {"index": 7, "name": "F1:+y|F1:-z", "start": [0.0, 600.0, 0.0], "end": [2000.0, 600.0, 0.0], "length": 2000.0},
        {"index": 9, "name": "F1:-x|F1:-y", "start": [0.0, 500.0, 0.0], "end": [0.0, 500.0, 2400.0], "length": 2400.0},
    ]});
    let faces = json!({"faces": [
        {"index": 0, "name": "F1:+z", "plane_normal": [0.0, 0.0, 1.0]},
        {"index": 1, "name": "F1:+y", "plane_normal": [0.0, 1.0, 0.0]},
        {"index": 3, "name": "F1:-y", "plane_normal": [0.0, -1.0, 0.0]},
        {"index": 4, "name": "F1:-x", "plane_normal": [-1.0, 0.0, 0.0]},
        {"index": 5, "name": "F1:-z", "plane_normal": [0.0, 0.0, -1.0]},
    ]});
    (edges, faces)
}

#[test]
fn an_edge_or_a_face_faces_away_from_the_wall() {
    let (edges, faces) = wall();
    let back = from_edge(&edges, &faces, 7).unwrap();
    assert_eq!(back.front, [0.0, 1.0]);
    // the face's bottom edge, at the floor, not its top one
    let front = from_face(&edges, &faces, 3).unwrap();
    assert_eq!(
        (front.a[2], front.b[2], front.front),
        (0.0, 0.0, [0.0, -1.0])
    );
    assert!(from_edge(&edges, &faces, 9).unwrap_err().contains("level"));
    assert!(from_face(&edges, &faces, 0)
        .unwrap_err()
        .contains("upright"));
}

#[test]
fn placed_with_its_back_on_the_line() {
    // along X, facing -Y: the back at y = 500, so the middle half the depth in front
    let line = from_sketch(&sketch([200.0, 500.0], [1400.0, 500.0], false, &[]), "l1").unwrap();
    let f = facing(&line, false);
    assert_eq!(
        place(&line, f, "left", 0.0, 600.0, 560.0),
        ([500.0, 220.0, 0.0], 0)
    );
    assert_eq!(place(&line, f, "left", 50.0, 600.0, 560.0).0[0], 550.0);
    assert_eq!(place(&line, f, "right", 100.0, 600.0, 560.0).0[0], 1000.0);
    assert_eq!(place(&line, f, "middle", 10.0, 600.0, 560.0).0[0], 810.0);
    // flipped it faces +Y, turned half: its left is now the line's +X end
    let back = facing(&line, true);
    assert_eq!(
        place(&line, back, "left", 0.0, 600.0, 560.0),
        ([1100.0, 780.0, 0.0], 180)
    );
    // along Y facing +X: a quarter turn, its left end at the near (-Y) end
    let wall = from_sketch(
        &sketch([0.0, 0.0], [0.0, 2000.0], false, &[[3000.0, 0.0]]),
        "l1",
    )
    .unwrap();
    let f = facing(&wall, false);
    assert_eq!(
        place(&wall, f, "left", 0.0, 600.0, 560.0),
        ([280.0, 300.0, 0.0], 90)
    );
    let w = facing(&wall, true);
    assert_eq!(
        place(&wall, w, "left", 0.0, 600.0, 560.0),
        ([-280.0, 1700.0, 0.0], -90)
    );
}

#[test]
fn width_from_line_and_free_dimension_names() {
    let line = from_sketch(&sketch([200.0, 500.0], [1400.0, 500.0], false, &[]), "l1").unwrap();
    assert_eq!(width(&line, "left", 50.0), 1150.0);
    assert_eq!(width(&line, "middle", 50.0), 1100.0);
    let design = json!({"params": [{"name": "Wall_line_l1"}]});
    assert_eq!(dimension_name("Wall line", "l1", &design), "Wall_line_l1_2");
    assert!(unnamed("d12") && !unnamed("d") && !unnamed("Wall_line_l1") && !unnamed("dx"));
}

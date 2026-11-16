use super::*;
use serde_json::json;

#[test]
fn finds_the_cabinet_of_a_selected_part() {
    let body = |name: &str, x: f64| json!({"name": name, "bbox": {"min": [x, 0, 0], "max": [x + 18.0, 560, 720]}});
    let measure = json!({"bodies": [
        body("Base Side L", 0.0),
        body("Base Side R", 582.0),
        body("Base Door", 0.0),
        body("Base 2 Side L", 600.0),
        body("Base 2 Side R", 1182.0),
        body("Base 2 Door", 600.0),
        body("Worktop", 0.0),
    ]});
    // a face of "Base 2 Door": the longer name, not "Base"
    let design = json!({"selection": [{"type": "face", "body": "Base 2 Door", "index": 3}]});
    let found = cabinet(&design, &measure).unwrap();
    assert_eq!(found.name, "Base 2");
    assert_eq!((found.min[0], found.max[0]), (600.0, 1200.0));
    assert_eq!(
        parts(&measure, "Base"),
        ["Base Side L", "Base Side R", "Base Door"]
    );
    assert_eq!(picked(&design), ["Base 2 Door"]);

    let design = json!({"selection": [{"type": "body", "name": "Worktop"}]});
    assert!(cabinet(&design, &measure).is_err());
    assert!(cabinet(&json!({"selection": []}), &measure).is_err());
}

fn stands(sides: [Plan; 2], back: Plan) -> Cabinet {
    let both = |i: usize, pick: fn(f64, f64) -> f64| pick(sides[0][i], sides[1][i]);
    Cabinet {
        name: "A".into(),
        min: [both(0, f64::min), both(1, f64::min), 0.0],
        max: [both(2, f64::max), both(3, f64::max), 720.0],
        sides,
        back,
    }
}

#[test]
fn a_cabinet_goes_against_the_side_it_is_placed_on() {
    // a box cabinet square to the axes: 600 wide, back at y = 280
    let base = stands(
        [[0.0, -280.0, 18.0, 280.0], [582.0, -280.0, 600.0, 280.0]],
        [18.0, 262.0, 582.0, 268.0],
    );
    assert_eq!(base.seat("right", 400.0, 560.0), ([800.0, 0.0, 0.0], 0));
    assert_eq!(base.seat("left", 400.0, 560.0), ([-200.0, 0.0, 0.0], 0));
    // on top: left sides and backs flush
    assert_eq!(base.seat("above", 400.0, 300.0), ([200.0, 130.0, 720.0], 0));

    // a right-hand corner cabinet, 900 x 900 and 560 deep: Side R closes its return, which
    // runs forward along the right wall (x = 450) and ends at y = -450
    let corner = stands(
        [
            [-450.0, -110.0, -432.0, 450.0],
            [-110.0, -450.0, 450.0, -432.0],
        ],
        [-432.0, 432.0, 450.0, 450.0],
    );
    // right of it: turned to face into the room, back on the right wall
    assert_eq!(
        corner.seat("right", 400.0, 560.0),
        ([170.0, -650.0, 0.0], -90)
    );
    // left of it: square, along the back wall
    assert_eq!(corner.seat("left", 400.0, 560.0), ([-650.0, 170.0, 0.0], 0));
    // what holds a placed cabinet to it: forward from the end of the return, half the return
    // from the corner cabinet's middle; to the left from the end of its back leg, half its width
    assert_eq!(corner.toward("right"), ([0.0, -1.0, 0.0], "return"));
    assert_eq!(corner.toward("left"), ([-1.0, 0.0, 0.0], "width"));
    assert_eq!(corner.toward("above"), ([0.0, 0.0, 1.0], "height"));
    // a box cabinet is its width across, either way
    assert_eq!(base.toward("right"), ([1.0, 0.0, 0.0], "width"));
    assert_eq!(base.toward("left"), ([-1.0, 0.0, 0.0], "width"));
    // on top of it: a corner cabinet of the same footprint, not turned
    assert_eq!(corner.seat("above", 900.0, 900.0), ([0.0, 0.0, 720.0], 0));

    // the cabinet now standing at the end of that return, facing into the room
    let turned = stands(
        [
            [-110.0, -468.0, 450.0, -450.0],
            [-110.0, -850.0, 450.0, -832.0],
        ],
        [444.0, -832.0, 450.0, -468.0],
    );
    // the run carries on along the wall, and so does a corner cabinet: turned too, its own
    // back in the next corner of the room
    assert_eq!(
        turned.seat("right", 400.0, 560.0),
        ([170.0, -1050.0, 0.0], -90)
    );
    assert_eq!(turned.toward("right"), ([0.0, -1.0, 0.0], "width"));
    assert_eq!(
        turned.seat("right", 900.0, 900.0),
        ([0.0, -1300.0, 0.0], -90)
    );
    // left of it is back towards the corner
    assert_eq!(turned.seat("left", 400.0, 560.0).0[1], -250.0);
    // on top: turned the same way, left sides (y = -450) and backs flush
    assert_eq!(
        turned.seat("above", 400.0, 300.0),
        ([300.0, -650.0, 720.0], -90)
    );

    // the mirror image: a left-hand corner cabinet's return is continued on its left
    let corner = stands(
        [
            [-450.0, -450.0, 110.0, -432.0],
            [432.0, -110.0, 450.0, 450.0],
        ],
        [-450.0, 432.0, 432.0, 450.0],
    );
    assert_eq!(
        corner.seat("left", 400.0, 560.0),
        ([-170.0, -650.0, 0.0], 90)
    );
    assert_eq!(corner.seat("right", 400.0, 560.0), ([650.0, 170.0, 0.0], 0));
}

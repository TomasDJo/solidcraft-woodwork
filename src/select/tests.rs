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

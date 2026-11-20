use super::*;

/// The commands a run of `entry` calls, against an app that answers every command with `{}`.
fn calls(entry: &str) -> Vec<String> {
    let mut called = Vec::new();
    let mut run = |command: &str, _: Value| {
        called.push(command.to_string());
        Ok(json!({}))
    };
    let _ = execute(entry, &json!({}), &mut run);
    called
}

#[test]
fn a_run_that_builds_a_cabinet_frames_the_model() {
    for entry in ["cabinet", "corner"] {
        let called = calls(entry);
        // Fit frames the selection, so it is cleared first.
        assert_eq!(
            called[called.len() - 2..],
            ["select.clear", "view.fit"],
            "{entry}"
        );
        assert_eq!(called.iter().filter(|c| *c == "view.fit").count(), 1);
    }
    // Material and the cut list change nothing worth framing.
    assert!(!calls("material").contains(&"view.fit".to_string()));
    assert!(!calls("cutlist").contains(&"view.fit".to_string()));
}

#[test]
fn prices_writes_only_what_is_filled_and_answers_what_is_stored() {
    let run_with = |params: Value| {
        let mut called = Vec::new();
        let mut run = |command: &str, args: Value| {
            called.push((command.to_string(), args));
            Ok(json!({"document": {"worktop": 150}}))
        };
        let reply = execute("prices", &params, &mut run).unwrap();
        (called, reply)
    };
    let (called, reply) = run_with(json!({"worktop": 150}));
    assert_eq!(
        called[0],
        ("storage.patch".into(), json!({"patch": {"worktop": 150}}))
    );
    assert_eq!(called[1].0, "storage.get");
    assert_eq!(reply, json!({"prices": {"worktop": 150}}));
    // Nothing filled: nothing written.
    let (called, _) = run_with(json!({}));
    assert_eq!(called.len(), 1);
    assert_eq!(called[0].0, "storage.get");
}

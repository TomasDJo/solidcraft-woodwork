use super::*;

#[test]
fn patch_keeps_only_what_was_set_in_mm() {
    let params = json!({"units": "in", "material": "MDF", "thickness": 0.75, "reveal": 3,
        "handles": "keep", "plinth_type": "keep"});
    assert_eq!(
        patch(&params, &Value::Null).unwrap(),
        json!({"settings": {"material": "MDF", "thickness": 19.05, "reveal": 3}})
    );
    let params = json!({"units": "mm", "plinth": 100, "plinth_type": "legs"});
    assert_eq!(
        patch(&params, &Value::Null).unwrap(),
        json!({"settings": {"plinth": 100.0, "plinth_type": "legs"}})
    );
    assert!(patch(&json!({"thickness": "thick"}), &Value::Null).is_err());
}

#[test]
fn lengths_out_of_bounds_are_refused_and_skipped() {
    let refused = |params: Value, field: &str| {
        let error = patch(&params, &Value::Null).unwrap_err();
        assert!(error.contains(field), "{error}");
    };
    refused(json!({"thickness": 3}), "thickness"); // under 0.125 in
    refused(json!({"units": "in", "thickness": 2}), "thickness"); // 50.8 mm
    refused(json!({"plinth": -1}), "plinth");
    refused(json!({"units": "in", "plinth": 20}), "plinth"); // 508 mm
    let edges = json!({"thickness": 3.175, "plinth": 500});
    assert!(patch(&edges, &Value::Null).is_ok());
    assert!(patch(&json!({"thickness": 50, "plinth": 0}), &Value::Null).is_ok());
    let stored = json!({"document": {"settings": {"thickness": 2, "plinth": 600,
        "material": "MDF"}}});
    let reply = fields("cabinet", &json!({"units": "in"}), &stored);
    assert_eq!(reply.as_object().unwrap().len(), 1, "{reply}");
    let stored = json!({"document": {"settings": {"thickness": 51, "plinth": -1}}});
    assert_eq!(fields("cabinet", &json!({}), &stored), json!({}));
}

#[test]
fn fields_are_defaults_in_the_dialogs_unit() {
    let stored = json!({"document": {"settings": {"material": "MDF", "thickness": 19.05,
        "reveal": 3, "plinth": 100, "other": 1}}});
    let hint = "From Woodwork settings";
    let mm = fields("cabinet", &json!({"units": "mm"}), &stored);
    assert_eq!(mm["thickness"], json!({"default": 19.05, "hint": hint}));
    assert_eq!(mm["material"]["default"], "MDF");
    assert!(mm.get("other").is_none());
    let inch = fields("corner", &json!({"units": "in"}), &stored);
    assert_eq!(inch["thickness"]["default"], 0.75);
    assert_eq!(inch["plinth"]["default"], 3.937008);
    assert_eq!(inch["reveal"]["default"], 3); // mm in every unit
    assert_eq!(fields("edit", &json!({}), &stored), json!({}));
    let nothing = json!({"document": null});
    assert_eq!(fields("cabinet", &json!({}), &nothing), json!({}));
}

#[test]
fn settings_dialog_numbers_are_stored_or_cabinet_defaults() {
    let stored = json!({"document": {"settings": {"material": "MDF", "reveal": 3}}});
    let mm = fields("settings", &json!({"units": "mm"}), &stored);
    assert_eq!(
        mm,
        json!({"thickness": {"default": 18.0}, "reveal": {"default": 3},
        "plinth": {"default": 0.0}})
    );
    let inch = fields(
        "settings",
        &json!({"units": "in"}),
        &json!({"document": null}),
    );
    assert_eq!(inch["thickness"]["default"], 0.708661);
}

#[test]
fn patch_leaves_out_numbers_as_stored_or_default() {
    let stored = json!({"document": {"settings": {"plinth": 100}}});
    let params = json!({"units": "in", "plinth": 3.937008, "reveal": 2, "thickness": 0.75});
    assert_eq!(
        patch(&params, &stored).unwrap(),
        json!({"settings": {"thickness": 19.05}})
    );
}

#[test]
fn camera_is_stored_and_only_leave_skips_the_fit() {
    let leave = patch(&json!({"camera": "leave"}), &Value::Null).unwrap();
    assert_eq!(leave, json!({"settings": {"camera": "leave"}}));
    assert_eq!(
        patch(&json!({"camera": "keep"}), &Value::Null).unwrap(),
        json!({"settings": {}})
    );
    assert!(!fits(&json!({"document": leave})));
    assert!(fits(&json!({"document": {"settings": {"camera": "fit"}}})));
    assert!(fits(&json!({})));
    // Not a Cabinet field, so the Cabinet dialog gets no default for it.
    let stored = json!({"document": {"settings": {"camera": "leave"}}});
    assert_eq!(fields("cabinet", &json!({}), &stored), json!({}));
}

#[test]
fn joinery_is_a_default_and_corner_cabinets_have_nothing_inside() {
    let stored = json!({"document": {"settings": {"joints": "confirmat"}}});
    let reply = fields("cabinet", &json!({}), &stored);
    assert_eq!(reply["joints"]["default"], "confirmat");
    let stored = json!({"document": {"settings": {"joints": "none", "joints_inner": "none"}}});
    let corner = fields("corner", &json!({}), &stored);
    assert_eq!(
        corner,
        json!({"joints": {"default": "none", "hint": "From Woodwork settings"}})
    );
    let patched = patch(&json!({"joints": "none"}), &Value::Null).unwrap();
    assert_eq!(patched, json!({"settings": {"joints": "none"}}));
}

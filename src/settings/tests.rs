use super::*;

#[test]
fn patch_keeps_only_what_was_set_in_mm() {
    let params = json!({"units": "in", "material": "MDF", "thickness": 0.75, "reveal": 3,
        "handles": "keep", "plinth_type": "keep"});
    assert_eq!(
        patch(&params, &Value::Null).unwrap(),
        json!({"settings": {"material": "MDF", "thickness": 19.05, "reveal": 3.0}})
    );
    let params = json!({"units": "mm", "plinth": 150, "plinth_type": "legs"});
    assert_eq!(
        patch(&params, &Value::Null).unwrap(),
        json!({"settings": {"plinth": 150.0, "plinth_type": "legs"}})
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
    // the plinth height is Settings' own now: Cabinet has Yes or No
    assert!(inch.get("plinth").is_none());
    assert_eq!(inch["reveal"]["default"], 3.0); // mm in every unit
    assert_eq!(fields("edit", &json!({}), &stored), json!({}));
    let nothing = json!({"document": null});
    assert_eq!(fields("cabinet", &json!({}), &nothing), json!({}));
}

#[test]
fn settings_dialog_starts_at_stored_values_or_cabinet_defaults() {
    let stored = json!({"document": {"settings": {"material": "MDF", "reveal": 3,
        "joints": "cam", "dowel_d": 10, "hinge_angle": "165"}}});
    let mm = fields("settings", &json!({"units": "mm"}), &stored);
    assert_eq!(mm["thickness"], json!({"default": 18.0}));
    assert_eq!(mm["reveal"], json!({"default": 3.0}));
    assert_eq!(mm["joints"], json!({"default": "cam"}));
    assert_eq!(mm["joints_inner"], json!({"default": "dowels"}));
    assert_eq!(mm["dowel_d"], json!({"default": 10.0}));
    assert_eq!(mm["joint_spacing"], json!({"default": 250.0}));
    assert_eq!(mm["hinge_angle"], json!({"default": "165"}));
    assert_eq!(mm["slide_type"], json!({"default": "full"}));
    // material keeps its Keep
    assert!(mm.get("material").is_none());
    assert_eq!(mm.as_object().unwrap().len(), table().len(), "{mm}");
    assert_eq!(mm["plinth"], json!({"default": 100.0}));
    assert_eq!(mm["kerf"], json!({"default": 4.0}));
    assert_eq!(mm["worktop_material"], json!({"default": "Oak"}));
    assert_eq!(mm["see_through"], json!({"default": "off"}));
    let inch = fields(
        "settings",
        &json!({"units": "in"}),
        &json!({"document": null}),
    );
    assert_eq!(inch["thickness"]["default"], 0.708661);
    assert_eq!(inch["joint_spacing"]["default"], 9.84252);
}

#[test]
fn design_fills_new_cabinets_from_the_settings() {
    let stored = json!({"document": {"settings": {"joints": "dado", "joints_inner": "none",
        "dowel_d": 10, "joint_spacing": 254, "hardware_models": "off", "slide_type": "over"}}});
    let mut p = json!({"units": "in"});
    design("cabinet", &mut p, &stored);
    assert_eq!(p["joints"], "dado");
    assert_eq!(p["joints_inner"], "none");
    assert_eq!(p["joint_spacing"], 10.0);
    assert_eq!(p["dowel_d"], 0.393701);
    assert_eq!(p["hardware_models"], "off");
    assert_eq!(p["slide_type"], "over");
    assert_eq!(p["hinge_angle"], "110");
    assert_eq!(p["biscuit"], "20");
    assert_eq!(p["fastener_models"], "on");
    assert!(p.get("see_through").is_none(), "no cabinet's");
    // a corner: no dado, nothing inside, no drawers
    let mut p = json!({});
    design("corner", &mut p, &stored);
    assert_eq!(p["joints"], "dowels");
    assert!(
        p.get("joints_inner").is_none() && p.get("slide_type").is_none(),
        "{p}"
    );
    assert_eq!(p["dowel_d"], 10.0);
    // nothing stored: today's defaults; a given key is kept
    let mut p = json!({"joints": "none"});
    design("cabinet", &mut p, &json!({}));
    assert_eq!(p["joints"], "none");
    assert_eq!(p["joints_inner"], "dowels");
    assert_eq!(p["cam_bolt"], 35.0);
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
fn joinery_is_design_wide_not_a_cabinet_default() {
    let stored = json!({"document": {"settings": {"joints": "confirmat", "material": "MDF"}}});
    let reply = fields("cabinet", &json!({}), &stored);
    assert_eq!(reply.as_object().unwrap().len(), 1, "{reply}");
    let patched = patch(&json!({"joints": "none"}), &Value::Null).unwrap();
    assert_eq!(patched, json!({"settings": {"joints": "none"}}));
    // the default, or what is stored, is left out
    let same = json!({"joints": "dowels", "dowel_d": 8, "hinge_angle": "keep"});
    assert_eq!(patch(&same, &Value::Null).unwrap(), json!({"settings": {}}));
    assert_eq!(
        patch(&json!({"joints": "confirmat"}), &stored).unwrap(),
        json!({"settings": {}})
    );
    let inch = patch(&json!({"units": "in", "dowel_d": 0.25}), &Value::Null).unwrap();
    assert_eq!(inch, json!({"settings": {"dowel_d": 6.35}}));
    assert!(patch(&json!({"dowel_d": 25}), &Value::Null).is_err());
}

#[test]
fn plinth_yes_takes_settings_no_takes_none_and_the_rest_is_filled() {
    let stored = json!({"document": {"settings": {"plinth": 150, "plinth_type": "legs",
        "fronts": "inset", "labels": "yes", "back_thickness": 8, "kerf": 3,
        "worktop_thickness": 40, "cut_sizes": "cut"}}});
    let mut p = json!({"units": "in", "plinth": "yes"});
    design("cabinet", &mut p, &stored);
    assert_eq!(p["plinth"], 5.905512);
    assert_eq!(
        (&p["plinth_type"], &p["fronts"]),
        (&json!("legs"), &json!("inset"))
    );
    assert_eq!(p["labels"], true);
    assert_eq!(p["back_thickness"], 0.314961);
    assert_eq!(p["plinth_setback"], 1.968504);
    let mut p = json!({"plinth": "no"});
    design("corner", &mut p, &stored);
    assert_eq!(p["plinth"], 0.0);
    assert!(p.get("back").is_none(), "a corner has no back choice");
    // the cut list and a worktop
    let mut p = json!({});
    design("cutlist", &mut p, &stored);
    assert_eq!(
        (&p["kerf"], &p["size"], &p["sheet"]),
        (&json!(3.0), &json!("cut"), &json!("2440x1220"))
    );
    assert!(p.get("joints").is_none());
    let mut p = json!({"units": "mm", "material": "MDF"});
    design("worktop", &mut p, &stored);
    assert_eq!(
        (&p["thickness"], &p["material"]),
        (&json!(40.0), &json!("MDF"))
    );
}

#[test]
fn edit_plinth_yes_keeps_a_plinth_and_gives_one_from_the_settings() {
    let remembered = json!({"params": {"name": "A", "units": "mm"}});
    let stored = json!({"document": {"settings": {"plinth": 120, "plinth_type": "notch"}}});
    let design = |mm: f64| json!({"params": [{"name": "A_plinth", "value": mm}]});
    let yes = json!({"plinth": "yes", "plinth_type": "keep", "shelves": 2});
    let got = edit_plinth(&yes, &remembered, &design(0.0), &stored);
    assert_eq!(
        (&got["plinth"], &got["plinth_type"]),
        (&json!(120.0), &json!("notch"))
    );
    let got = edit_plinth(&yes, &remembered, &design(80.0), &stored);
    assert!(
        got.get("plinth").is_none() && got["plinth_type"] == "keep",
        "{got}"
    );
    let no = edit_plinth(
        &json!({"plinth": "no"}),
        &remembered,
        &design(80.0),
        &stored,
    );
    assert_eq!(no["plinth"], 0);
    let keep = edit_plinth(
        &json!({"plinth": "keep"}),
        &remembered,
        &design(80.0),
        &stored,
    );
    assert!(keep.get("plinth").is_none());
}

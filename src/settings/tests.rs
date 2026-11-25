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
    let mm = fields(
        "settings",
        &json!({"units": "mm", "section": "all"}),
        &stored,
    );
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
    assert_eq!(p["fastener_models"], "off");
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

#[test]
fn apply_brings_a_cabinet_to_the_settings_and_skips_one_that_has_them() {
    let stored = json!({"document": {"settings": {"thickness": 19, "joints": "confirmat",
        "plinth": 120, "plinth_type": "legs"}}});
    let mut built = json!({"units": "mm", "thickness": 18.0, "plinth": 0.0});
    design("cabinet", &mut built, &json!({}));
    let (p, changed) = apply("cabinet", &built, &stored).unwrap();
    // (a cabinet without a plinth keeps its plinth type)
    assert_eq!(changed, ["joints", "thickness"]);
    assert_eq!(
        (&p["thickness"], &p["joints"]),
        (&json!(19.0), &json!("confirmat"))
    );
    // no plinth: it keeps none
    assert_eq!(p["plinth"], 0.0);
    assert_eq!(apply("cabinet", &p, &stored), None);
    // with one: the settings' height and type
    let mut on = built.clone();
    on["plinth"] = json!(100.0);
    let (p, _) = apply("cabinet", &on, &stored).unwrap();
    assert_eq!(
        (&p["plinth"], &p["plinth_type"]),
        (&json!(120.0), &json!("legs"))
    );
    // a corner takes no drawers' or inside settings
    let mut corner = json!({"units": "mm", "plinth": 0.0});
    design("corner", &mut corner, &json!({}));
    let (p, _) = apply("corner", &corner, &stored).unwrap();
    assert!(
        p.get("slide_type").is_none() && p.get("joints_inner").is_none(),
        "{p}"
    );
}

#[test]
fn apply_takes_the_hardware_models_both_ways() {
    let mut built = json!({"units": "mm", "plinth": 0.0});
    design("cabinet", &mut built, &json!({}));
    assert_eq!(built["hardware_models"], "on");
    let off = json!({"document": {"settings": {"hardware_models": "off"}}});
    let (p, changed) = apply("cabinet", &built, &off).unwrap();
    assert_eq!(p["hardware_models"], "off");
    assert_eq!(changed, ["hardware_models"]);
    let (back, _) = apply("cabinet", &p, &json!({})).unwrap();
    assert_eq!(back["hardware_models"], "on");
    // a cabinet made before the key existed (absent) gets it
    let mut old = built.clone();
    old.as_object_mut().unwrap().remove("hardware_models");
    assert!(apply("cabinet", &old, &off).is_some());
}

#[test]
fn match_selected_is_yes_with_a_cabinet_picked_and_hidden_otherwise() {
    let design = json!({"bodies": ["A Side L", "A Top", "Loose"],
        "selection": []});
    let pick = |name: &str| json!({"selection": [{"type": "face", "body": name, "index": 2}]});
    let yes = json!({"default": "selected", "visible": true});
    assert_eq!(matching(&pick("A Top"), &design), yes);
    assert_eq!(
        matching(&json!({"selection": []}), &design),
        json!({"visible": false})
    );
    assert_eq!(matching(&pick("Loose"), &design), json!({"visible": false}));
    // the design's selection when the field is empty, as for a script
    let selected =
        json!({"bodies": ["A Side L"], "selection": [{"type": "body", "name": "A Side L"}]});
    assert_eq!(matching(&json!({}), &selected), yes);
}

#[test]
fn match_selected_shows_the_picked_cabinets_sizes() {
    let design = json!({"bodies": ["Cabinet 2 Side L", "Cabinet 2 Top"], "selection": [],
        "params": [{"name": "Cabinet_2_height", "value": 720.0},
            {"name": "Cabinet_2_depth", "value": 560.0},
            {"name": "Cabinet_2_thickness", "value": 19.05},
            {"name": "Cabinet_2_plinth", "value": 100.0}]});
    let pick = json!({"units": "in", "selection": [{"type": "body", "name": "Cabinet 2 Top"}]});
    let sizes = matched_sizes(&pick, &design);
    let hint = "Match selected takes it from Cabinet 2";
    assert_eq!(sizes["thickness"], json!({"default": 0.75, "hint": hint}));
    assert_eq!(sizes["plinth"], json!({"default": "yes", "hint": hint}));
    assert_eq!(sizes["height"]["default"], 28.346457);
    assert!(matched_sizes(&json!({"selection": []}), &design).is_empty());
}

#[test]
fn a_section_shows_its_fields_and_hides_the_rest_still_sent() {
    let joinery = fields("settings", &json!({"section": "joinery"}), &json!({}));
    assert!(joinery["joints"].get("visible").is_none());
    assert_eq!(joinery["joints"]["default"], "dowels");
    // hidden, but with its value: the hook hides without leaving it out
    assert_eq!(
        joinery["plinth"],
        json!({"default": 100.0, "visible": false})
    );
    assert_eq!(joinery["material"], json!({"visible": false}));
    assert!(joinery.get("units").is_none());
    let all = fields("settings", &json!({"section": "all"}), &json!({}));
    assert!(all
        .as_object()
        .unwrap()
        .values()
        .all(|f| f.get("visible").is_none()));
    // every setting and Settings' own keep-choices are in one section
    let mut keys: Vec<&str> = SECTIONS.iter().flat_map(|s| s.1.iter().copied()).collect();
    let mut wanted = kept();
    keys.sort();
    wanted.retain(|k| *k != "handles");
    wanted.push("handles");
    wanted.sort();
    assert_eq!(keys, wanted);
    // Section is no setting: it is not stored
    let patched = patch(
        &json!({"section": "joinery", "joints": "none"}),
        &Value::Null,
    )
    .unwrap();
    assert_eq!(patched, json!({"settings": {"joints": "none"}}));
}

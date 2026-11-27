use super::*;

#[test]
fn patch_keeps_only_what_was_set_in_mm() {
    let params = json!({"units": "in", "material": "MDF", "thickness": 0.75, "reveal": 3,
        "handles": "keep", "plinth_type": "keep"});
    assert_eq!(
        patch(&params, &Value::Null).unwrap(),
        // Units is kept too: the other sections use it
        json!({"settings": {"material": "MDF", "thickness": 19.05, "reveal": 3.0, "units": "in"}})
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
    // the stored material, the Cabinet dialog's handles and the camera's Fit: never Keep
    assert_eq!(mm["material"], json!({"default": "MDF"}));
    assert_eq!(mm["handles"], json!({"default": "none"}));
    assert_eq!(mm["camera"], json!({"default": "fit"}));
    let empty = fields(
        "settings",
        &json!({"section": "all"}),
        &json!({"document": null}),
    );
    assert_eq!(empty["material"], json!({"default": "Birch plywood"}));
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
    // the strip runners' sizes (#146): stored, or 12 x 18 mm; box cabinets only
    assert_eq!(
        (p["strip_thickness"].as_f64(), p["strip_height"].as_f64()),
        (Some(12.0), Some(18.0))
    );
    let strips = json!({"document": {"settings": {"slide_type": "strips", "strip_thickness": 15, "strip_height": 22}}});
    let mut p = json!({});
    design("cabinet", &mut p, &strips);
    assert_eq!(
        (p["strip_thickness"].as_f64(), p["strip_height"].as_f64()),
        (Some(15.0), Some(22.0))
    );
    let mut p = json!({});
    design("corner", &mut p, &strips);
    assert!(p.get("strip_thickness").is_none(), "{p}");
}

#[test]
fn patch_leaves_out_numbers_as_stored_or_default() {
    let stored = json!({"document": {"settings": {"plinth": 100}}});
    let params = json!({"units": "in", "plinth": 3.937008, "reveal": 2, "thickness": 0.75});
    assert_eq!(
        patch(&params, &stored).unwrap(),
        json!({"settings": {"thickness": 19.05, "units": "in"}})
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
    assert_eq!(inch, json!({"settings": {"dowel_d": 6.35, "units": "in"}}));
    // a section without Units gives its sizes in the stored one
    let stored_in = json!({"document": {"settings": {"units": "in"}}});
    let section = patch(&json!({"dowel_d": 0.25}), &stored_in).unwrap();
    assert_eq!(section, json!({"settings": {"dowel_d": 6.35}}));
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
    assert_eq!(p["double_dividers"], false);
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
    // with one: the settings' height and type (it had the default type)
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
fn apply_follows_changed_defaults_and_keeps_what_was_chosen() {
    // made with the built-in defaults, pull handles and no plinth chosen for it
    let mut built = json!({"units": "mm", "plinth": 0.0, "handles": "pull", "wall_mount": "cleat"});
    design("cabinet", &mut built, &json!({}));
    built["_defaults"] = defaults("cabinet", &json!("mm"), &json!({}));
    let stored = json!({"document": {"settings": {"handles": "knob", "fronts": "inset",
        "back": "none", "handle_position": "top", "labels": "yes", "plinth_type": "legs",
        "joints": "confirmat"}}});
    let (p, mut changed) = apply("cabinet", &built, &stored).unwrap();
    changed.sort();
    // what still had the old default follows the settings
    assert_eq!(
        changed,
        ["back", "fronts", "handle_position", "joints", "labels"]
    );
    assert_eq!(
        (&p["fronts"], &p["labels"]),
        (&json!("inset"), &json!(true))
    );
    // its own values stay: pull handles, no plinth (so its plinth type too), the cleat
    for key in ["handles", "plinth", "plinth_type", "wall_mount"] {
        assert_eq!(p[key], built[key], "{key}");
    }
    assert_eq!(p["_defaults"], defaults("cabinet", &json!("mm"), &stored));
    // one with the default plinth takes the new plinth type
    let mut base = built.clone();
    base["plinth"] = json!(100.0);
    let (p, _) = apply("cabinet", &base, &stored).unwrap();
    assert_eq!(
        (&p["plinth"], &p["plinth_type"]),
        (&json!(100.0), &json!("legs"))
    );
    // one made before 0.5.0 (no `_defaults`) is taken as made with the built-in defaults
    let mut old = base.clone();
    old.as_object_mut().unwrap().remove("_defaults");
    assert_eq!(
        apply("cabinet", &old, &stored).unwrap().0["plinth_type"],
        "legs"
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
fn a_section_dialog_gets_only_its_fields() {
    let joinery = fields("settings-joinery", &json!({}), &json!({}));
    let mut keys: Vec<&str> = joinery
        .as_object()
        .unwrap()
        .keys()
        .map(|k| k.as_str())
        .collect();
    keys.sort();
    let mut wanted = SECTIONS
        .iter()
        .find(|s| s.0 == "joinery")
        .unwrap()
        .1
        .to_vec();
    wanted.sort();
    assert_eq!(keys, wanted);
    assert_eq!(joinery["joints"]["default"], "dowels");
    // a size outside General says its unit, set there; General has Units
    assert_eq!(joinery["dowel_d"]["hint"], "In mm, as set in General");
    let stored_in = json!({"document": {"settings": {"units": "in", "dowel_d": 6.35}}});
    let inch = fields("settings-joinery", &json!({}), &stored_in);
    assert_eq!(inch["dowel_d"]["default"], 0.25);
    let general = fields("settings-general", &json!({"units": "mm"}), &stored_in);
    assert_eq!(general["units"], json!({"default": "in"}));
    // Finger-joint the bottom too goes by the stored plinth type, set in another dialog
    assert!(joinery["finger_bottom"].get("visible").is_none());
    for (plinth, shown) in [
        ("board", true),
        ("notch", true),
        ("legs", false),
        ("casters", false),
    ] {
        let stored = json!({"document": {"settings": {"plinth_type": plinth}}});
        let reply = fields("settings-joinery", &json!({}), &stored);
        assert_eq!(
            reply["finger_bottom"].get("visible").is_none(),
            shown,
            "{plinth}"
        );
    }
    // every setting is in one section
    let mut keys: Vec<&str> = SECTIONS.iter().flat_map(|s| s.1.iter().copied()).collect();
    let mut wanted = kept();
    keys.sort();
    wanted.sort();
    assert_eq!(keys, wanted);
    // the scripts' entry keeps every setting
    let all = fields("settings", &json!({}), &json!({}));
    assert!(all.get("joints").is_some() && all.get("plinth").is_some());
    // nothing is stored but what a run gives
    let patched = patch(&json!({"joints": "none"}), &Value::Null).unwrap();
    assert_eq!(patched, json!({"settings": {"joints": "none"}}));
}

#[test]
fn a_refusal_names_the_field_it_is_about() {
    let field = |why: &str, corner: bool| problem(why, corner).0;
    assert_eq!(
        field(
            "doors must be 0, 1, 2 or one per compartment (dividers + 1 = 1 here)",
            false
        ),
        "doors"
    );
    assert_eq!(
        field("doors must be none, hinged left or hinged right", true),
        "corner_doors"
    );
    assert_eq!(field("too low for that many drawers", false), "height");
    assert_eq!(
        field("doors with hinge cups need panels of 16 mm or more", false),
        "thickness"
    );
    assert_eq!(field("dividers must be 0 to 10", false), "dividers");
    assert_eq!(
        field("height is too small for the plinth and shelves", false),
        "height"
    );
    assert_eq!(
        field("width is too small for the panels and dividers", false),
        "width"
    );
    assert_eq!(field("something new", false), "shape");
    // a setting's field is said on the Cabinet field it belongs to
    assert_eq!(
        on_cabinet(field("cleat thickness must be 6 to 50 mm", false)),
        "wall_mount"
    );
    assert_eq!(problem("depth is too small", false).1, "Depth is too small");
    // a size out of bounds is said on its own field
    let (f, _) = trouble(&json!({"thickness": 60}), &json!({})).unwrap();
    assert_eq!(f, "thickness");
    assert!(trouble(&json!({"thickness": 19}), &json!({})).is_none());
}

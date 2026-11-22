//! Board materials: made in the design on first use, then assigned to bodies.

use crate::Run;
use serde_json::{json, Value};

/// Name, density in g/cm3, and look: an appearance from the app's library, or a colour.
/// Oak is the app's own material. The manifest lists the same names.
const BOARDS: [(&str, f64, (&str, &str)); 4] = [
    (
        "Birch plywood",
        0.68,
        ("appearance", "Wood - Cream Birch Uniform Preview"),
    ),
    ("MDF", 0.75, ("color", "#b59a73")),
    ("White melamine", 0.68, ("color", "#f1f1ec")),
    ("Oak", 0.75, ("appearance", "Wood - Oak")),
];

/// The materials of `BOARDS` with a grain: their parts must not be turned on the sheet.
pub const GRAINED: [&str; 2] = ["Birch plywood", "Oak"];

/// Our boards' opacity with Woodwork settings' See-through boards on: the fasteners and fittings
/// inside show through.
pub const SEE_THROUGH: f64 = 0.35;

/// The opacity Woodwork settings ask for, from `storage.get`'s reply.
pub fn opacity(stored: &Value) -> f64 {
    match stored["document"]["settings"]["see_through"] == "on" {
        true => SEE_THROUGH,
        false => 1.0,
    }
}

pub fn assign(run: Run, material: &str, bodies: &[String]) -> Result<(), String> {
    let (name, density, look) = (BOARDS.iter().find(|board| board.0 == material))
        .ok_or(format!("unknown material `{material}`"))?;
    let list = run("material.list", json!({}))?;
    let list = list.get("materials").unwrap_or(&list);
    if !list
        .as_array()
        .is_some_and(|all| all.iter().any(|m| m["name"] == *name))
    {
        let mut args = json!({"name": name, "density_g_cm3": density});
        args[look.0] = json!(look.1);
        // made see-through when the boards already made are
        let opacity = opacity(&run("storage.get", json!({}))?);
        if opacity < 1.0 {
            args["opacity"] = json!(opacity);
            args["pbr"] = Value::Null; // a library look's shading is opaque only
        }
        run("material.create", args)?;
    }
    run(
        "material.assign",
        json!({"bodies": bodies, "material": name}),
    )?;
    Ok(())
}

/// Gives our boards' materials in the design `opacity` (`material.edit`, which takes it only
/// with the look, so that is given again), each that has another. One the app has built in, such
/// as Oak, cannot be changed and stays solid. Returns the materials changed.
pub fn see_through(run: Run, opacity: f64) -> Result<Vec<String>, String> {
    let list = run("material.list", json!({}))?;
    let list = list.get("materials").unwrap_or(&list);
    let mut changed = Vec::new();
    for m in list.as_array().into_iter().flatten() {
        let Some((name, _, look)) = BOARDS.iter().find(|b| m["name"] == b.0) else {
            continue;
        };
        let now = m["appearance"]["opacity"].as_f64().unwrap_or(1.0);
        if m["user"] != true || (now - opacity).abs() < 1e-6 {
            continue;
        }
        let mut args = json!({"material": name, "opacity": opacity});
        args[look.0] = json!(look.1);
        // a library look's shading (PBR) is opaque only: plain shading while see-through, its
        // own again (`pbr` left out) when solid
        if opacity < 1.0 {
            args["pbr"] = Value::Null;
        }
        run("material.edit", args)?;
        changed.push(name.to_string());
    }
    Ok(changed)
}

//! Board materials: made in the design on first use, then assigned to bodies.

use crate::Run;
use serde_json::json;

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
        run("material.create", args)?;
    }
    run(
        "material.assign",
        json!({"bodies": bodies, "material": name}),
    )?;
    Ok(())
}

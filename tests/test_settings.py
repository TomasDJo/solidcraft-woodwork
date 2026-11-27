"""Woodwork settings: defaults kept in the design, that Cabinet and Corner start from."""
import json
import subprocess

from build import CLI, entry_step
from helpers import *


def without(step, *keys):
    """A plugin.run step with `keys` left out, for the dialog hook to fill in."""
    for key in keys:
        step["params"]["params"].pop(key)
    return step


def test_settings(approval):
    kept = ("material", "thickness")
    results = cli([plugin(approval, "settings", {"material": "MDF", "thickness": 25, "plinth": 100,
                                                 "plinth_type": "legs"}),
                   plugin(approval, "settings", {"units": "in", "thickness": 1}),  # only this changes
                   without(plugin(approval, "cabinet", {"name": "A", "plinth": "yes"}), *kept),
                   without(plugin(approval, "cabinet", {"name": "B", "units": "in", "place": "origin",
                                                        "plinth": "yes"}), *kept),
                   plugin(approval, "corner", {"name": "C", "material": "Oak", "thickness": 18, "plinth": "no"}),
                   MEASURE])
    stored = results[1].get("result", results[1])["settings"]
    # 100 is the default; Units is kept for the other sections
    assert stored == {"material": "MDF", "thickness": 25.4, "plinth_type": "legs", "units": "in"}, stored
    box = boxes(results[5])
    material = {b["name"]: b["material"] for b in results[5]["bodies"]}
    # A from the settings: MDF, 25.4 mm sides, on 100 mm legs (Plinth Yes).
    assert material["A Side L"] == "MDF", material
    assert round(box["A Side L"][3] - box["A Side L"][0], 3) == 25.4 and box["A Side L"][2] == 100, box["A Side L"]
    assert "A Leg 1" in box, box
    # B's dialog in inches gets them in inches: the same boards.
    assert round(box["B Side L"][3] - box["B Side L"][0], 3) == 25.4 and box["B Side L"][2] == 100, box["B Side L"]
    # What a run gives wins: C is oak, 18 mm, on the floor (Plinth No).
    assert material["C Side L"] == "Oak", material
    assert box["C Side L"][2] == 0 and "C Leg 1" not in box, box


def test_camera_leave_skips_the_fit(approval):
    # Camera `leave` in Woodwork settings: a cabinet run reports no view; back to `fit`, it does.
    results = cli([plugin(approval, "settings", {"camera": "leave"}),
                   plugin(approval, "cabinet", {"name": "A"}),
                   plugin(approval, "settings", {"camera": "fit"}),
                   plugin(approval, "cabinet", {"name": "B"})])
    assert results[0].get("result", results[0])["settings"] == {"camera": "leave"}, results[0]
    assert [results[i].get("view") for i in (1, 3)] == [None, {"kind": "fit"}], results


def test_joinery_and_hardware_are_design_wide(approval):
    # Stored: confirmat 500 apart, hinges to 165, no models. A new cabinet takes them and
    # remembers them; Edit keeps them after the settings changed.
    results = cli([setting(approval, {"joints": "confirmat", "joint_spacing": 500, "hinge_angle": "165",
                                      "hardware_models": "off"}),
                   plugin(approval, "cabinet", {"name": "A", "doors": 1, "shelves": 0}),
                   setting(approval, {"joints": "none"}),
                   select("A Side L"), plugin(approval, "edit", {"labels": "yes"}),
                   MEASURE, {"command": "component.metadata", "params": {"component": "A"}},
                   {"command": "joint.list", "params": {}}])
    stored = results[0].get("result", results[0])["settings"]
    assert stored == {"joints": "confirmat", "joint_spacing": 500, "hinge_angle": "165", "hardware_models": "off"}, stored
    assert results[2].get("result", results[2])["settings"]["joints"] == "none"
    kept = results[6]["metadata"]["io.github.tomasdjo.woodwork"]["params"]
    assert (kept["joints"], kept["joint_spacing"], kept["hinge_angle"], kept["hardware_models"]) == ("confirmat", 500, "165", "off"), kept
    # 460 mm between the first and last, at most 500 apart: 2 screws a joint
    assert cylinders(results[5], "A Bottom") == 4, cylinders(results[5], "A Bottom")
    assert not [b for b in results[5]["bodies"] if b["name"].startswith("HW hinge (")]
    limits = {j["name"]: j["values"][0]["limits"] for j in results[7]["joints"]}
    assert limits["A Door hinge"] == [0, 165], limits


def test_a_cabinet_run_takes_no_joinery(approval):
    # Settings only: Cabinet has conditions, so a key it does not declare is refused.
    step = plugin(approval, "cabinet", {"name": "A"})
    step["params"]["params"]["joints"] = "dowels"
    script = ROOT / "dist" / "settings-refused.json"
    script.write_text(json.dumps([step]), encoding="utf-8")
    done = subprocess.run([CLI, "run", str(script), "--quiet"], capture_output=True, text=True)
    # (the app names no field)
    assert done.returncode and "is not an input of this entry" in done.stderr, done.stderr


def test_see_through_boards(approval):
    # On: the birch plywood of A made before, and the MDF of B made after, are see-through;
    # oak, built into the app, stays solid. Off: solid again.
    def opacity(listed):
        return {m["name"]: m.get("appearance", {}).get("opacity", 1) for m in listed["materials"]}
    results = cli([plugin(approval, "cabinet", {"name": "A", "doors": 1}),
                   setting(approval, {"see_through": "on"}),
                   plugin(approval, "cabinet", {"name": "B", "material": "MDF", "place": "origin"}),
                   plugin(approval, "cabinet", {"name": "C", "material": "Oak", "place": "origin"}),
                   {"command": "material.list", "params": {}},
                   setting(approval, {"see_through": "off"}),
                   {"command": "material.list", "params": {}}],
                  "--save", str(ROOT / "dist" / "see-through.sc"))
    assert results[1]["result"]["see_through"] == ["Birch plywood"], results[1]
    seen = opacity(results[4])
    assert seen["Birch plywood"] == seen["MDF"] == 0.35 and seen["Oak"] == 1, seen
    assert results[5]["result"]["see_through"] == ["Birch plywood", "MDF"], results[5]
    seen = opacity(results[6])
    assert seen["Birch plywood"] == seen["MDF"] == 1, seen


def test_plinth_fronts_cut_list_and_worktop_from_the_settings(approval):
    # Settings: a 120 mm plinth on legs, inset fronts, no back, 2800 x 2070 sheets, a 30 mm
    # worktop. A takes them (Plinth Yes); W, a wall unit, has Plinth No; W gets the settings'
    # plinth by Edit (Plinth Yes), and A keeps its own when the settings change.
    results = cli([setting(approval, {"plinth": 120, "plinth_type": "legs", "fronts": "inset", "back": "none",
                                      "sheet": "2800x2070", "worktop_thickness": 30}),
                   entry_step(approval, "cabinet", {"name": "A", "doors": 1, "plinth": "yes"}),
                   entry_step(approval, "cabinet", {"name": "W", "plinth": "no", "place": "origin"}),
                   MEASURE,
                   setting(approval, {"plinth": 80}),
                   select("W Side L"), plugin(approval, "edit", {"plinth": "yes"}),
                   select("A Side L"), plugin(approval, "edit", {"plinth": "yes", "shelves": 2}),
                   MEASURE, plugin(approval, "cutlist"),
                   select("A Top"), plugin(approval, "worktop"), MEASURE])
    box = boxes(results[3])
    assert box["A Side L"][2] == 120 and "A Leg 1" in box and "A Back" not in box, box["A Side L"]
    assert box["A Door"][1] == box["A Side L"][1], (box["A Door"], box["A Side L"])  # inset: flush
    assert box["W Side L"][2] == 0 and "W Leg 1" not in box
    box = boxes(results[9])
    assert box["W Side L"][2] == 80 and "W Leg 1" in box, box["W Side L"]  # the settings' now
    assert box["A Side L"][2] == 120 and "A Shelf 2" in box, box["A Side L"]  # its own kept
    assert results[10]["result"]["sheet_mm"] == [2800, 2070], results[10]["result"]["sheet_mm"]
    worktop = boxes(results[13])["Worktop"]
    assert round(worktop[5] - worktop[2], 3) == 30, worktop


def test_apply_settings_to_picked(approval):
    # A, B and W made with the defaults; Settings with 19 mm boards and biscuits, then Apply
    # settings with A and W picked: they are built again with them, in place, B is not. W, a
    # wall unit without a plinth, keeps none though the settings have one. Run again: nothing
    # left to do. Settings alone only stores them.
    def pick(*names):
        return [{"type": "body", "name": n} for n in names]
    new = {"thickness": 19, "joints": "biscuit", "plinth": 120}
    light = {"shelves": 0, "depth": 300}
    results = cli([plugin(approval, "cabinet", {"name": "A", "plinth": "yes", **light}), select("A Side R"),
                   plugin(approval, "cabinet", {"name": "B", "place": "right", "plinth": "yes", **light}),
                   plugin(approval, "cabinet", {"name": "W", "plinth": "no", "place": "origin", **light}),
                   MEASURE, setting(approval, new),
                   plugin(approval, "rebuild", {"selection": pick("A Top", "W Side L")}),
                   MEASURE, plugin(approval, "cutlist"),
                   plugin(approval, "rebuild", {"selection": pick("A Top", "W Side L")}),
                   setting(approval, {"thickness": 25}), MEASURE])
    before, after = boxes(results[4]), boxes(results[7])
    reply = results[6]["result"]
    assert sorted(reply["applied"]) == ["A", "W"], reply
    width = lambda box, name: round(box[f"{name} Side L"][3] - box[f"{name} Side L"][0], 3)
    assert width(after, "A") == width(after, "W") == 19 and width(after, "B") == 18, after
    assert after["A Bottom"][2] == 120 and after["W Bottom"][2] == 0, (after["A Bottom"], after["W Bottom"])
    assert after["B Side L"] == before["B Side L"] and after["A Side L"][0] == before["A Side L"][0]
    hardware = {h["item"]: h["qty"] for h in results[8]["result"]["hardware"]}
    assert hardware["Biscuit, #20"] == 16 and "Dowel, 8 x 35 mm" in hardware, hardware
    assert results[9]["result"]["applied"] == [], results[9]
    assert "applied" not in results[10]["result"] and boxes(results[11])["B Side L"] == before["B Side L"], results[10]


def test_apply_settings_keeps_what_is_per_cabinet(approval):
    # W, a wall cabinet on a French cleat without a plinth, and B, a base cabinet with one and
    # pull handles. Settings with confirmat, a 120 plinth and knobs, then Apply settings to all
    # (none picked): both get confirmat, B the 120 plinth; W keeps no plinth and its cleat, B
    # its pull handles (handles are per cabinet).
    light = {"shelves": 0, "depth": 300}
    results = cli([plugin(approval, "cabinet", {"name": "W", "plinth": "no", "wall_mount": "cleat", "place": "origin", **light}),
                   plugin(approval, "cabinet", {"name": "B", "plinth": "yes", "doors": 1, "handles": "pull", "place": "origin", **light}),
                   setting(approval, {"joints": "confirmat", "plinth": 120, "handles": "knob"}),
                   plugin(approval, "rebuild", {}), MEASURE])
    reply = results[3]["result"]
    assert sorted(reply["applied"]) == ["B", "W"], reply
    assert "joints" in reply["changed"]["W"] and "joints" in reply["changed"]["B"], reply["changed"]
    assert "plinth" not in reply["changed"]["W"] and "handles" not in reply["changed"]["B"], reply["changed"]
    box = boxes(results[4])
    assert box["W Bottom"][2] == 0 and "W Cleat" in box and "W Wall cleat" in box, box["W Bottom"]
    assert box["B Bottom"][2] == 120, box["B Bottom"]


def test_apply_settings_keeps_an_edit_to_no_plinth(approval):
    # A made with a plinth, then edited to none. Settings with confirmat and a 120 plinth, then
    # Apply settings to all: A gets confirmat and still has no plinth. Its plinth parameter used
    # to stay at 100 after the Edit, and Apply settings read the sizes from the parameters.
    light = {"shelves": 0, "depth": 300}
    results = cli([plugin(approval, "cabinet", {"name": "A", "plinth": "yes", "place": "origin", **light}),
                   select("A Side L"), plugin(approval, "edit", {"plinth": "no"}),
                   setting(approval, {"joints": "confirmat", "plinth": 120}),
                   plugin(approval, "rebuild", {}), MEASURE])
    reply = results[4]["result"]
    assert reply["applied"] == ["A"] and "plinth" not in reply["changed"]["A"], reply
    box = boxes(results[5])
    assert box["A Bottom"][2] == 0 and "A Plinth" not in box, box["A Bottom"]


def test_apply_settings_follows_a_changed_default(approval):
    # A, B and C on the default board plinth, W a wall cabinet with no plinth chosen. Settings
    # Plinth type Legs, then Apply settings to all: the three still had the old default, so they
    # get legs; W keeps no plinth (not the default when it was made).
    light = {"shelves": 0, "depth": 300}
    script = [plugin(approval, "cabinet", {"name": n, "plinth": "yes", "place": "origin", **light}) for n in "ABC"]
    script += [plugin(approval, "cabinet", {"name": "W", "plinth": "no", "place": "origin", **light}),
               setting(approval, {"plinth_type": "legs"}), plugin(approval, "rebuild", {}), MEASURE]
    results = cli(script)
    reply = results[5]["result"]
    assert all("plinth_type" in reply["changed"][n] for n in "ABC"), reply
    box = boxes(results[6])
    for n in "ABC":
        assert f"{n} Leg 1" in box and f"{n} Plinth" not in box, sorted(box)
    assert box["W Bottom"][2] == 0 and not any(b.startswith("W Leg") for b in box), box["W Bottom"]


def test_apply_hardware_models_both_ways(approval):
    # A with doors and a drawer, models shown. Settings Hardware models None, applied with a
    # hinge model picked (what a user clicks to turn the models off): no HW copies left in A,
    # and the reply names the change. Shown again with a board picked: they come back. Once
    # more: A already has the settings, skipped and said so.
    def copies(results, i):
        return [b["name"] for b in results[i]["bodies"] if b["name"].startswith("HW ") and "(A:1" in b["name"]]
    made = cli([plugin(approval, "cabinet", {"name": "A", "doors": 1, "drawers": 1}), MEASURE],
               "--save", str(ROOT / "dist" / "apply-models.sc"))
    hinge = next(n for n in copies(made, 1) if n.startswith("HW hinge ("))
    on = [{"type": "body", "name": "A Side L"}]
    results = cli([setting(approval, {"hardware_models": "off"}),
                   plugin(approval, "rebuild", {"selection": [{"type": "body", "name": hinge}]}), MEASURE,
                   setting(approval, {"hardware_models": "on"}), plugin(approval, "rebuild", {"selection": on}), MEASURE,
                   plugin(approval, "rebuild", {"selection": on})],
                  "--in", str(ROOT / "dist" / "apply-models.sc"))
    off = results[1]["result"]
    assert off["applied"] == ["A"] and off["changed"]["A"] == ["hardware_models"], off
    assert not copies(results, 2), copies(results, 2)
    assert results[4]["result"]["applied"] == ["A"] and copies(results, 5), results[4]["result"]
    again = results[6]["result"]
    assert again["applied"] == [] and again["skipped"] == [{"cabinet": "A", "why": "already has these settings"}], again
    assert "1 skipped" in again["message"], again["message"]


def test_rebuild_job_takes_more_than_one_run(approval):
    # Apply settings (a 0.48 job), none picked: all twelve corner cabinets, made without
    # joinery, get confirmat from the settings in one run, which takes longer than an ordinary
    # run's 30 s; then As made builds the picked one again as it now is.
    import time
    saved = ROOT / "dist" / "rebuild-job.sc"
    script = [setting(approval, {"joints": "none", "hardware_models": "off"})]
    script += [plugin(approval, "corner", {"name": f"C{i}", "place": "origin"}) for i in range(12)]
    cli(script, "--save", str(saved))
    start = time.time()
    results = cli([setting(approval, {"joints": "confirmat"}), plugin(approval, "rebuild", {}),
                   plugin(approval, "rebuild", {"with": "made", "selection": [{"type": "body", "name": "C0 Top"}]}),
                   {"command": "joint.list", "params": {}}], "--in", str(saved))
    elapsed = time.time() - start
    reply = results[1]["result"]
    assert len(reply["applied"]) == 12, reply
    assert all(c == ["joints"] for c in reply["changed"].values()), reply["changed"]
    assert elapsed > 30, elapsed
    again = results[2]["result"]
    assert again["applied"] == ["C0"] and "as they were made" in again["message"], again


def test_a_section_dialog_keeps_the_other_sections(approval):
    # A dialog per section (`settings-<section>`, the Woodwork settings dropdown): a run of the
    # Display dialog changes the camera and keeps the stored joinery and plinth; the scripts'
    # `settings` entry still takes any field.
    results = cli([setting(approval, {"joints": "confirmat", "plinth": 150}),
                   plugin(approval, "settings-display", {"camera": "leave"}),
                   setting(approval, {"joints": "cam"})])
    stored = results[1]["result"]["settings"]
    assert stored == {"joints": "confirmat", "plinth": 150, "camera": "leave"}, stored
    assert results[2]["result"]["settings"]["joints"] == "cam", results[2]


def test_drawer_settings_in_inches(approval):
    # Drawer settings with General in inches (#238): the hook starts Tab width at 120 mm in
    # inches (4.72), which the dialog's bounds must take, and it is kept as 120 mm.
    results = cli([plugin(approval, "settings", {"units": "in"}),
                   entry_step(approval, "settings-drawers", {"drawer_bottom": 0.25}),
                   plugin(approval, "settings", {"units": "mm", "drawer_bottom": 6})])
    stored = results[1].get("result", results[1])["settings"]
    assert round(stored["drawer_bottom"], 2) == 6.35, stored
    assert round(stored.get("tab_width", 120), 3) == 120, stored

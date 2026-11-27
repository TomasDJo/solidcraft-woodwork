"""Build, package and end-to-end test the plugin.

    python build.py            build dist/woodwork/ and dist/woodwork-<version>.zip, then test
    python build.py --install  also install the zip into SolidCraft
    python build.py corner     only the end-to-end tests with "corner" in their file or name

The end-to-end tests run in parallel, min(4, CPUs) at a time; WOODWORK_TEST_JOBS=1 runs them one by one.

The end-to-end tests are in tests/, one file per function of the plugin.

Needs: cargo (with the wasm32-unknown-unknown target), wasm-tools, solidcraft-cli (or the path to it
in SOLIDCRAFT_CLI).
"""
import json
import os
import shutil
import subprocess
import sys
import tempfile
import threading
import traceback
import zipfile
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path

ROOT = Path(__file__).parent
PKG = ROOT / "dist" / "woodwork"
os.environ["PATH"] += os.pathsep + str(Path.home() / ".cargo" / "bin")
CLI = os.environ.get("SOLIDCRAFT_CLI") or shutil.which("solidcraft-cli") or r"C:\Program Files\SolidCraft\solidcraft-cli.exe"
# Per test thread: the environment its solidcraft-cli runs get (helpers.own_profile sets it).
LOCAL = threading.local()


def env():
    return getattr(LOCAL, "env", None)


def sh(*args):
    subprocess.run(args, cwd=ROOT, check=True)


def flat(script):
    """The steps of a script, a `plugin` step that came as a group (settings, entry, settings) as
    its three."""
    return [step for group in script for step in (group if isinstance(group, tuple) else (group,))]


def cli(script, *extra):
    """Run a command script through solidcraft-cli and return the results: one a step, a group's
    the result of its entry."""
    with tempfile.TemporaryDirectory(dir=ROOT / "dist") as folder:
        path, out = Path(folder) / "script.json", Path(folder) / "results.json"
        path.write_text(json.dumps(flat(script)), encoding="utf-8")
        subprocess.run([CLI, "run", str(path), "--results", "--quiet", "--json-out", str(out), *extra],
                       cwd=ROOT, check=True, env=env())
        results = iter(json.loads(out.read_text(encoding="utf-8")))
        return [[next(results) for _ in group][1] if isinstance(group, tuple) else next(results)
                for group in script]


# The fields Woodwork settings keep for the whole design (no field of these entries any more):
# the entry's name for each, the setting's, and the setting's default (lengths in mm).
MOVED = {
    "cabinets": {"back": ("back", "groove"), "back_thickness": ("back_thickness", 6), "fronts": ("fronts", "overlay"),
                 "handle_position": ("handle_position", "auto"), "plinth_type": ("plinth_type", "board"),
                 "plinth_setback": ("plinth_setback", 50), "labels": ("labels", "no"),
                 "cleat_height": ("cleat_height", 90), "cleat_thickness": ("cleat_thickness", 18),
                 "drawer_thickness": ("drawer_thickness", 12), "drawer_bottom": ("drawer_bottom", 6),
                 "front_style": ("front_style", "flat"), "groove_shape": ("groove_shape", "square"),
                 "groove_spacing": ("groove_spacing", 50), "groove_depth": ("groove_depth", 3)},
    "cutlist": {"banding": ("banding", "1"), "size": ("cut_sizes", "finished"), "sheet": ("sheet", "2440x1220"),
                "kerf": ("kerf", 4), "trim": ("trim", 10)},
    "worktop": {"material": ("worktop_material", "Oak"), "thickness": ("worktop_thickness", 38)},
}

# Fields Edit had that are design-wide now (Woodwork settings), with the setting's default.
DESIGN_WIDE = {"joints": "dowels", "joints_inner": "dowels", "hardware_models": "on", "hinge_angle": "110",
               "slide_type": "full", "reveal": 2, "plinth_setback": 50,
               "groove_shape": "square", "groove_spacing": 50, "groove_depth": 3}


def plugin(approval, entry, params=None):
    """A plugin.run step. The fields left out get their defaults, as in the dialog: from 0.46 an
    entry with conditional fields is checked strictly, every visible required field present.
    A field Woodwork settings keep now (`MOVED`) goes to a Settings run before the step and back
    to its default after it, a group `cli` runs as one step; a cabinet's plinth height too,
    the cabinet then Plinth Yes. A cabinet the test gives no plinth stands on the floor."""
    params = dict(params or {})
    # Corner cabinets are Cabinet with a corner shape (its doors are `corner_doors`).
    if entry == "corner":
        entry = "cabinet"
        params = {"shape": "L-shaped", **params}
        if "doors" in params:
            params["corner_doors"] = params.pop("doors")
    # Edit has only what is per cabinet: a design-wide field is Woodwork settings', then Rebuild
    # cabinets with the settings for the picked one, and the setting back to its default.
    if entry == "edit":
        wide = {k: params.pop(k) for k in DESIGN_WIDE if k in params}
        if wide:
            if "plinth_setback" in wide:  # shown with a board or notch only
                wide.setdefault("plinth_type", "board")
            reset = {k: DESIGN_WIDE.get(k, "board") for k in wide}
            again = {"with": "settings"}  # no cabinets picked in its field: all of them
            steps = [entry_step(approval, "settings", wide | {"units": "mm"})]
            if set(params) - {"selection"}:
                steps.append(entry_step(approval, "edit", params))
            steps.append(entry_step(approval, "rebuild", again))
            return (*steps, entry_step(approval, "settings", reset | {"units": "mm"}))
    moved = MOVED.get("cabinets" if entry in ("cabinet", "corner") else entry, {})
    setting = {}
    for field, (key, _) in moved.items():
        if field in params:
            value = params.pop(field)
            setting[key] = ("yes" if value else "no") if key == "labels" else value
    if entry in ("cabinet", "corner") and not isinstance(params.get("plinth", 0), str):
        height = params.get("plinth", 0)
        if height:
            setting["plinth"] = height
        params["plinth"] = "yes" if height else "no"
    step = entry_step(approval, entry, params)
    if not setting:
        return step
    # A conditional setting needs the field that shows it (the app checks that first).
    for field, controller, value in (("plinth_setback", "plinth_type", "board"), ("back_thickness", "back", "groove"),
                                     ("cut_sizes", "banding", "1")):
        if field in setting:
            setting.setdefault(controller, value)
    reset = {key: default for key, default in moved.values() if key in setting}
    if "plinth" in setting:
        reset["plinth"] = 100
    units = params.get("units", "mm")
    return (entry_step(approval, "settings", setting | {"units": units}), step,
            entry_step(approval, "settings", reset | {"units": "mm"}))


def holds(condition, values):
    """Whether a dialog condition (`{field, eq}`, `{field, in}`, `all`, `any`, `not`) holds."""
    if "field" in condition:
        value = values.get(condition["field"])
        return value == condition["eq"] if "eq" in condition else value in condition["in"]
    (key, inner), = condition.items()
    if key == "not":
        return not holds(inner, values)
    return (all if key == "all" else any)(holds(c, values) for c in inner)


def entry_step(approval, entry, params=None):
    """The plugin.run step itself, with the schema's defaults. Edit is Cabinet with Mode Edit
    picked, whose hook starts every field at the picked cabinet's value."""
    if entry == "edit":
        params = {"mode": "edit", "selection": [], **(params or {})}
        if isinstance(params.get("shelf_pins"), str):  # Cabinet's is a tick box
            params["shelf_pins"] = params["shelf_pins"] == "yes"
        return {"command": "plugin.run", "params": {"path": str(PKG), "approval": approval, "entry": "cabinet", "params": params}}
    manifest = json.loads((ROOT / "plugin.json").read_text(encoding="utf-8"))
    schema = next(e["input_schema"] for e in manifest["runtime"]["entries"] if e["name"] == entry)
    # Woodwork settings' hook starts its fields at the stored values, as for plugin.run: all
    # but these, which start at Keep.
    defaults = {key: field.get("default", []) for key, field in schema["properties"].items()
                if ("default" in field or field["type"] == "array")
                and (entry != "settings" or key in ("units", "material", "handles", "camera"))}
    # Each field's conditional default when its rule holds, as the dialog does (sizes in inches).
    given = defaults | (params or {})
    for key, field in schema["properties"].items():
        for rule in field.get("x-solidcraft-defaults", []):
            if holds(rule["when"], given):
                defaults[key] = rule["value"]
                break
    # Cabinet's Mode Auto goes by what is picked: the tests say New unless they test Auto.
    if entry == "cabinet":
        defaults["mode"] = "new"
    # The tests name their parts after the plugin's own fallback name, not the dialog's.
    if "name" in defaults and entry != "worktop":
        defaults["name"] = "Cabinet"
    return {"command": "plugin.run", "params": {
        "path": str(PKG), "approval": approval, "entry": entry, "params": defaults | (params or {})}}


def workshop():
    """Bring solidcraft-workshop.json (the Workshop listing) up to plugin.json's version and its
    package, and the SolidCraft version CI tests against as the minimum. Not on CI, where
    test_manifest must catch a listing that was committed stale."""
    if os.environ.get("CI"):
        return
    path = ROOT / "solidcraft-workshop.json"
    listing = json.loads(path.read_text(encoding="utf-8"))
    version = json.loads((ROOT / "plugin.json").read_text(encoding="utf-8"))["version"]
    ci = (ROOT / ".github" / "workflows" / "build.yml").read_text(encoding="utf-8")
    app = ci.split("SOLIDCRAFT:", 1)[1].split()[0]
    package = {"asset": f"woodwork-{version}.zip", "role": "package"}
    files = [package if f.get("role") == "package" else f for f in listing["files"]]
    updated = {**listing, "version": version, "min_app_version": app, "files": files}
    if updated != listing:
        path.write_text(json.dumps(updated, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
        print(f"solidcraft-workshop.json: version {version}, SolidCraft {app} or later")


def build():
    sh("cargo", "test", "--quiet")
    sh("cargo", "build", "--release", "--target", "wasm32-unknown-unknown")
    PKG.mkdir(parents=True, exist_ok=True)
    sh("wasm-tools", "component", "new",
       "target/wasm32-unknown-unknown/release/woodwork.wasm", "-o", str(PKG / "woodwork.wasm"))
    # SolidCraft 0.45 refuses a manifest over 64 KiB: the packaged one carries no white space.
    manifest = json.loads((ROOT / "plugin.json").read_text(encoding="utf-8"))
    (PKG / "plugin.json").write_text(json.dumps(manifest, ensure_ascii=False, separators=(",", ":")),
                                     encoding="utf-8", newline=chr(10))
    shutil.copy(ROOT / "LICENSE", PKG)
    shutil.copytree(ROOT / "icons", PKG / "icons", dirs_exist_ok=True)
    version = manifest["version"]
    workshop()
    archive = ROOT / "dist" / f"woodwork-{version}.zip"
    with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED) as z:
        icons = sorted(f"icons/{icon.name}" for icon in (ROOT / "icons").glob("*.svg"))
        for name in ("plugin.json", "woodwork.wasm", "LICENSE", *icons):
            z.write(PKG / name, f"woodwork/{name}")
    # What Install would refuse: an error stops the build, a warning is only printed.
    sh(CLI, "plugin", "check", str(archive))
    return archive


def test(only=()):
    """Run the end-to-end tests in tests/test_*.py: every `test_*(approval)` function, or those
    whose file or name contains one of the words in `only`."""
    import importlib
    sys.path[:0] = [str(ROOT), str(ROOT / "tests")]
    approval = cli([{"command": "plugin.inspect", "params": {"path": str(PKG)}}])[0]["approval"]
    tests = {}
    for path in sorted((ROOT / "tests").glob("test_*.py")):
        module = importlib.import_module(path.stem)
        for name, function in vars(module).items():
            label = f"{path.stem}.{name}"
            if name.startswith("test_") and (not only or any(word in label for word in only)):
                tests[label] = function
    assert tests, f"no test matches {only}"

    def run(function):
        try:
            function(approval)
        except BaseException:
            return traceback.format_exc()

    jobs = int(os.environ.get("WOODWORK_TEST_JOBS") or min(4, os.cpu_count() or 1))
    failed = []
    with ThreadPoolExecutor(jobs) as pool:
        futures = {pool.submit(run, function): label for label, function in tests.items()}
        for future in as_completed(futures):
            label, error = futures[future], future.result()
            if error:
                failed.append(label)
                print(f"FAIL {label}\n{error}", flush=True)
            else:
                print("ok  ", label, flush=True)
    if failed:
        sys.exit(f"FAIL: {len(failed)} of {len(tests)} tests: {', '.join(sorted(failed))}")
    print("PASS" if not only else f"PASS ({len(tests)} of the tests)")


if __name__ == "__main__":
    archive = build()
    test([word for word in sys.argv[1:] if not word.startswith("-")])
    if "--install" in sys.argv:
        print(cli([{"command": "plugin.install", "params": {"path": str(archive), "replace": True}}]))

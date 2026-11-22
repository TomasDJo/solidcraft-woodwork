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


def cli(script, *extra):
    """Run a command script through solidcraft-cli and return the results."""
    with tempfile.TemporaryDirectory(dir=ROOT / "dist") as folder:
        path, out = Path(folder) / "script.json", Path(folder) / "results.json"
        path.write_text(json.dumps(script), encoding="utf-8")
        subprocess.run([CLI, "run", str(path), "--results", "--quiet", "--json-out", str(out), *extra],
                       cwd=ROOT, check=True, env=env())
        return json.loads(out.read_text(encoding="utf-8"))


def plugin(approval, entry, params=None):
    """A plugin.run step. The fields left out get their defaults, as in the dialog: from 0.46 an
    entry with conditional fields is checked strictly, every visible required field present."""
    manifest = json.loads((ROOT / "plugin.json").read_text(encoding="utf-8"))
    schema = next(e["input_schema"] for e in manifest["runtime"]["entries"] if e["name"] == entry)
    defaults = {key: field.get("default", []) for key, field in schema["properties"].items()
                if "default" in field or field["type"] == "array"}
    # Each field's conditional default when its rule holds, as the dialog does (sizes in inches).
    given = defaults | (params or {})
    for key, field in schema["properties"].items():
        for rule in field.get("x-solidcraft-defaults", []):
            if given.get(rule["when"]["field"]) == rule["when"]["eq"]:
                defaults[key] = rule["value"]
                break
    # The tests name their parts after the plugin's own fallback name, not the dialog's.
    if "name" in defaults:
        defaults["name"] = "Cabinet"
    return {"command": "plugin.run", "params": {
        "path": str(PKG), "approval": approval, "entry": entry, "params": defaults | (params or {})}}


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

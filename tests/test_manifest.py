"""The dialogs in plugin.json: only what SolidCraft 0.44 can draw, and every choice with a title."""
import json

from helpers import *

DRAWN = {"type", "title", "description", "default", "minimum", "maximum", "minLength", "maxLength", "oneOf", "maxItems"}


def test_choices_have_titles(approval):
    manifest = json.loads((ROOT / "plugin.json").read_text(encoding="utf-8"))
    choices = 0
    for entry in manifest["runtime"]["entries"]:
        # The dialog shows the fields in the order `required` names them, and the rest by name:
        # so every field is listed, in the order it is written here. A required field cannot be
        # left empty, so a field that may be empty (it has no default, or an empty one) is not.
        # The selection field is listed: with nothing in it, it sends an empty list.
        schema = entry["input_schema"]
        filled = [name for name, field in schema["properties"].items()
                  if field.get("default", "") != "" or field["type"] == "array"]
        assert schema.get("required", []) == filled, entry["name"]
        for name, field in schema["properties"].items():
            at = (entry["name"], name)
            # A keyword the dialog cannot draw leaves the entry with Run turned off.
            assert set(field) <= DRAWN, (at, set(field) - DRAWN)
            # The one list the dialog draws: `selection`, picked in the viewport (0.44). The app
            # reserves that name for bodies, faces and edges, which costs the manifest nothing;
            # faces matter, as a click on a board selects one. Another array, or more keywords
            # on this one, leaves Run turned off.
            assert (field["type"] == "array") == (name == "selection"), at
            if "oneOf" not in field:
                continue
            choices += 1
            options = field["oneOf"]
            values = [o["const"] for o in options]
            assert field["type"] == "string" and options and len(set(values)) == len(values), at
            assert field["default"] in values, at
            for o in options:
                assert set(o) <= {"const", "title", "description"} and isinstance(o["const"], str) and o.get("title", o["const"]), at
    assert choices == 29, choices
    # SolidCraft 0.44 refuses a manifest over 16 KiB (0.45: 64 KiB); the package carries it
    # without white space.
    packed = (PKG / "plugin.json").read_bytes()
    assert json.loads(packed) == manifest and len(packed) < 16 * 1024 - 100, len(packed)
    # Each function names its icon, and the package has it.
    for entry in manifest["runtime"]["entries"]:
        icon = entry["input_schema"]["x-solidcraft-icon"]
        assert icon == f"icons/{entry['name']}.svg" and (PKG / icon).is_file(), icon
    # The plugin still gets the value, not the title.
    results = cli([plugin(approval, "corner", {"shape": "diagonal", "doors": "hinged right"}), MEASURE])
    assert "Cabinet Door" in boxes(results[1]), boxes(results[1])

"""The dialogs in plugin.json: only what SolidCraft 0.43 can draw, and every choice with a title."""
import json

from helpers import *

DRAWN = {"type", "title", "description", "default", "minimum", "maximum", "minLength", "maxLength", "oneOf"}


def test_choices_have_titles(approval):
    manifest = json.loads((ROOT / "plugin.json").read_text(encoding="utf-8"))
    choices = 0
    for entry in manifest["runtime"]["entries"]:
        # The dialog shows the fields in the order `required` names them, and the rest by name:
        # so every field is listed, in the order it is written here. A required field cannot be
        # left empty, so a text field that may be empty is not listed.
        schema = entry["input_schema"]
        filled = [name for name, field in schema["properties"].items() if field.get("default") != ""]
        assert schema.get("required", []) == filled, entry["name"]
        for name, field in schema["properties"].items():
            at = (entry["name"], name)
            # A keyword the dialog cannot draw leaves the entry with Run turned off.
            assert set(field) <= DRAWN, (at, set(field) - DRAWN)
            if "oneOf" not in field:
                continue
            choices += 1
            options = field["oneOf"]
            values = [o["const"] for o in options]
            assert field["type"] == "string" and options and len(set(values)) == len(values), at
            assert field["default"] in values, at
            for o in options:
                assert set(o) <= {"const", "title", "description"} and isinstance(o["const"], str) and o["title"], at
    assert choices == 19, choices
    # The app refuses a manifest over 16 KiB; the package carries it without white space.
    packed = (PKG / "plugin.json").read_bytes()
    assert json.loads(packed) == manifest and len(packed) < 14 * 1024, len(packed)
    # The plugin still gets the value, not the title.
    results = cli([plugin(approval, "corner", {"shape": "diagonal", "doors": "hinged right"}), MEASURE])
    assert "Cabinet Door" in boxes(results[1]), boxes(results[1])

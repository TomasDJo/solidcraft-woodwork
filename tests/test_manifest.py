"""The dialogs in plugin.json: only what SolidCraft 0.46 can draw, and every choice with a title."""
import json

from helpers import *

DRAWN = {"type", "title", "description", "default", "minimum", "maximum", "minLength", "maxLength", "oneOf", "maxItems", "x-solidcraft-visible-when",
         "x-solidcraft-defaults"}
WHEN = "x-solidcraft-visible-when"


def conditions(condition):
    """The `{field, eq}` and `{field, in}` leaves of a condition, each with whether a `not` is over it."""
    if "field" in condition:
        assert set(condition) in ({"field", "eq"}, {"field", "in"}), condition
        return [(condition, False)]
    (key, inner), = condition.items()
    if key == "not":
        return [(leaf, not negated) for leaf, negated in conditions(inner)]
    assert key in ("all", "any") and isinstance(inner, list), condition
    return [leaf for c in inner for leaf in conditions(c)]


def fits(field, value):
    """Whether a value is one the field can have, so a condition on it can come true."""
    if "oneOf" in field:
        return value in [o["const"] for o in field["oneOf"]]
    kind = {"string": str, "boolean": bool, "integer": int, "number": (int, float)}[field["type"]]
    if not isinstance(value, kind) or (field["type"] != "boolean" and isinstance(value, bool)):
        return False
    if field["type"] == "string":
        return field.get("minLength", 0) <= len(value) <= field.get("maxLength", len(value))
    return field["type"] == "boolean" or field.get("minimum", value) <= value <= field.get("maximum", value)


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
            # A field shown only when it applies (0.46). Hidden, it is left out of Run and not
            # required; it still has a default for when it shows, as `required` orders it.
            # Its controllers are sibling scalar fields, and each value one they can have. A
            # hidden controller matches neither `eq` nor `in`, so `not` over a controller that
            # can hide comes true when it hides: write such a test as an `in` list instead.
            if WHEN in field:
                assert name not in schema["required"] or "default" in field, at
                for leaf, negated in conditions(field[WHEN]):
                    controller = schema["properties"][leaf["field"]]
                    assert leaf["field"] != name and controller["type"] in ("string", "boolean", "number", "integer"), at
                    values = [leaf["eq"]] if "eq" in leaf else leaf["in"]
                    assert values and all(fits(controller, v) for v in values), (at, leaf)
                    assert not (negated and WHEN in controller), (at, leaf)
            # A size's inch default (0.46): the dialog shows it when Units is Inches, until the
            # field is changed. Each one a size the field can have.
            for rule in field.get("x-solidcraft-defaults", []):
                assert rule["when"] == {"field": "units", "eq": "in"} and fits(field, rule["value"]), (at, rule)
            # Its description is the field's hover tooltip: every field explains itself.
            assert field.get("description", "").strip(), at
            # The one list the dialog draws: `selection`, picked in the viewport (0.44). The app
            # reserves that name for bodies, faces and edges, which costs the manifest nothing;
            # faces matter, as a click on a board selects one. Another array, or more keywords
            # on this one, leaves Run turned off.
            assert (field["type"] == "array") == (name == "selection"), at
            # The dialog keeps a number within its bounds; a value the plugin refuses past them
            # only stops the preview, with no message. The reveal is 0.5 to 6 mm in every unit.
            if name == "reveal":
                assert (field["minimum"], field["maximum"]) == (0.5, 6), at
            if "oneOf" not in field:
                continue
            choices += 1
            options = field["oneOf"]
            values = [o["const"] for o in options]
            assert field["type"] == "string" and options and len(set(values)) == len(values), at
            assert field["default"] in values, at
            for o in options:
                assert set(o) <= {"const", "title", "description"} and isinstance(o["const"], str) and o.get("title", o["const"]), at
    assert choices == 59, choices
    # SolidCraft 0.45 refuses a manifest over 64 KiB; the package carries it without white space.
    packed = (PKG / "plugin.json").read_bytes()
    assert json.loads(packed) == manifest and len(packed) < 64 * 1024, len(packed)
    # Each function names its icon, and the package has it.
    for entry in manifest["runtime"]["entries"]:
        icon = entry["input_schema"]["x-solidcraft-icon"]
        assert icon == f"icons/{entry['name']}.svg" and (PKG / icon).is_file(), icon
    # The plugin still gets the value, not the title.
    results = cli([plugin(approval, "corner", {"shape": "diagonal", "doors": "hinged right"}), MEASURE])
    assert "Cabinet Door" in boxes(results[1]), boxes(results[1])

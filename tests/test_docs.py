"""The usage guide's pictures: each one scripts/usage_pictures.py draws is in docs/images and
shown in docs/usage.md, and no picture there is left over."""
import importlib.util

from helpers import ROOT


def test_usage_pictures_are_drawn_and_shown(approval):
    spec = importlib.util.spec_from_file_location("usage_pictures", ROOT / "scripts" / "usage_pictures.py")
    pictures = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(pictures)
    usage = (ROOT / "docs" / "usage.md").read_text(encoding="utf-8")
    names = [shot[0] for shot in pictures.SHOTS]
    for name in names:
        assert (ROOT / "docs" / "images" / f"{name}.png").is_file(), f"{name}.png is not drawn"
        assert f"](images/{name}.png)" in usage, f"{name}.png is not in docs/usage.md"
    drawn = sorted(p.stem for p in (ROOT / "docs" / "images").glob("*.png"))
    assert drawn == sorted(names), drawn

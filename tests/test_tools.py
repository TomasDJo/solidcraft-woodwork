"""The functions as tools for agents (SolidCraft 0.45): listed by `plugin.tools`, called over MCP
with its strict argument check, and as the `woodwork.<entry>` command."""
import json
import subprocess

from build import CLI
from helpers import *

TYPICAL = {"name": "Agent", "width": 800, "height": 900, "depth": 400}


def outline(measure):
    """The box round every body: x0, y0, z0, x1, y1, z1 in mm."""
    corners = list(boxes(measure).values())
    return [round(f(c[i] for c in corners), 3) for f, i in ((min, 0), (min, 1), (min, 2), (max, 3), (max, 4), (max, 5))]


def mcp(*calls):
    """Runs a headless MCP server on these requests: lists its tools and makes each tool call.
    Returns the tools and the replies."""
    requests = [("initialize", {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "test", "version": "1"}}),
                ("tools/list", {})] + [("tools/call", {"name": name, "arguments": arguments}) for name, arguments in calls]
    text = "".join(json.dumps({"jsonrpc": "2.0", "id": n, "method": method, "params": params}) + "\n"
                   for n, (method, params) in enumerate(requests))
    out = subprocess.run([CLI, "mcp", "--headless"], input=text, capture_output=True, text=True,
                         encoding="utf-8", timeout=300).stdout
    replies = {reply.get("id"): reply for reply in map(json.loads, out.splitlines())}
    return replies[1]["result"]["tools"], [replies[n] for n in range(2, len(requests))]


def test_agent_tools(approval):
    with own_profile() as install:
        installed = cli([install])[0]
        schema = next(e["input_schema"] for e in installed["entry_points"] if e["name"] == "cabinet")
        # Every field of ours is required, which orders the dialog: so an agent passes them all,
        # the defaults with what it wants changed. Fewer is refused before the plugin runs.
        everything = {key: field.get("default", []) for key, field in schema["properties"].items()} | TYPICAL
        measure = ("measure", {})
        tools, (few, built, measured) = mcp(("woodwork.cabinet", TYPICAL), ("woodwork.cabinet", everything), measure)
        listed = {t["name"]: t for t in tools if t["name"].startswith("woodwork.")}
        names = ("cabinet", "corner", "edit", "joinery", "material", "worktop", "cutlist")
        assert sorted(listed) == sorted(f"woodwork.{n}" for n in names), sorted(listed)
        assert listed["woodwork.cabinet"]["inputSchema"]["required"] == schema["required"]
        assert listed["woodwork.cabinet"]["inputSchema"]["properties"]["width"] == schema["properties"]["width"]
        assert "missing required argument" in json.dumps(few), few
        assert not built["result"].get("isError"), built
        cabinet = json.loads(measured["result"]["content"][0]["text"])
        assert outline(cabinet) == [-400, -200, 0, 400, 200, 900], outline(cabinet)
        assert "Agent Side L" in boxes(cabinet)
        # The command an engine call or a script uses passes a few fields through unchecked.
        command = {"command": "woodwork.cabinet", "params": {"approval": installed["approval"], "params": TYPICAL}}
        results = cli([install, {"command": "plugin.tools", "params": {}}, command, MEASURE])
        tools = {t["name"]: t["input_schema"] for t in results[1]["tools"]}
        assert tools["woodwork.cabinet"]["required"] == schema["required"], tools.keys()
        assert outline(results[3]) == [-400, -200, 0, 400, 200, 900], outline(results[3])

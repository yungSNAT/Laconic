# Copyright 2026 Ryan Duguid
# SPDX-License-Identifier: Apache-2.0

"""Smoke tests for the laconic-check MCP servers over stdio: `laconic-check mcp` and the
deprecated Node `mcp/server.mjs`."""

import json
import os
import pathlib
import shutil
import subprocess
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
SERVER = ROOT / "tools/laconic-check/mcp/server.mjs"


def find_binary():
    # Prefer this checkout's build, so a stale installed copy cannot hide a regression.
    return (
        os.environ.get("LACONIC_CHECK")
        or shutil.which(ROOT / "tools/laconic-check/target/debug/laconic-check")
        or shutil.which("laconic-check")
    )


@unittest.skipUnless(
    find_binary(),
    "MCP smoke test not run: no laconic-check binary in LACONIC_CHECK, target/debug or the PATH",
)
class McpServerTest(unittest.TestCase):
    """`laconic-check mcp`."""

    def command(self):
        return [str(find_binary()), "mcp"]

    def exchange(self, *requests):
        """Send the requests and return the server's replies keyed by id."""
        result = subprocess.run(
            self.command(),
            input="".join(json.dumps({"jsonrpc": "2.0", **r}) + "\n" for r in requests),
            capture_output=True,
            encoding="utf-8",
            env={**os.environ, "LACONIC_CHECK": str(find_binary())},
            timeout=60,
            check=False,
        )
        self.assertEqual(0, result.returncode, result.stderr)
        replies = [json.loads(line) for line in result.stdout.splitlines()]
        return {reply["id"]: reply for reply in replies}

    def call(self, name, **arguments):
        reply = self.exchange(
            {"id": 1, "method": "tools/call", "params": {"name": name, "arguments": arguments}}
        )[1]
        self.assertNotIn("error", reply)
        return reply["result"]

    def test_initialise_then_list_tools(self):
        replies = self.exchange(
            {"id": 1, "method": "initialize", "params": {"protocolVersion": "2025-06-18"}},
            {"method": "notifications/initialized"},
            {"id": 2, "method": "tools/list"},
        )
        self.assertEqual({1, 2}, set(replies), "a notification must get no reply")
        self.assertEqual("laconic-check", replies[1]["result"]["serverInfo"]["name"])
        self.assertEqual("2025-06-18", replies[1]["result"]["protocolVersion"])
        tools = {tool["name"]: tool for tool in replies[2]["result"]["tools"]}
        self.assertEqual({"lint", "preserve"}, set(tools))
        self.assertEqual(["text"], tools["lint"]["inputSchema"]["required"])
        self.assertEqual(["original", "edited"], tools["preserve"]["inputSchema"]["required"])

    def test_lint_reports_a_finding_by_line(self):
        result = self.call("lint", text="Clean line.\n\nIt worked — mostly.\n")
        self.assertNotIn("isError", result)
        self.assertRegex(result["content"][0]["text"], r"(?m)^line 3:")

    def test_lint_reports_clean_text(self):
        result = self.call("lint", text="It worked.\n")
        self.assertEqual("No findings.", result["content"][0]["text"])

    def test_preserve_reports_a_dropped_number(self):
        result = self.call("preserve", original="Keep 42 items.\n", edited="Keep items.\n")
        self.assertNotIn("isError", result)
        self.assertIn("42", result["content"][0]["text"])

    def test_tool_errors_are_reported_in_the_result(self):
        result = self.call("lint", text=1)
        self.assertTrue(result["isError"])


@unittest.skipUnless(shutil.which("node"), "Node MCP smoke test not run: node is not on the PATH")
class NodeServerTest(McpServerTest):
    """The deprecated `mcp/server.mjs`, which runs the binary once a call."""

    def command(self):
        return ["node", str(SERVER)]


if __name__ == "__main__":
    unittest.main()

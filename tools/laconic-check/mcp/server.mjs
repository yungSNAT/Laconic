#!/usr/bin/env node
// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

// MCP server (stdio) exposing laconic-check as two tools: lint and preserve.
// No dependencies: it speaks newline-delimited JSON-RPC 2.0 and runs the
// laconic-check binary named by LACONIC_CHECK, used as given, or else found in PATH's fully
// qualified entries.
//
//   claude mcp add laconic-check -- node /path/to/tools/laconic-check/mcp/server.mjs

import { spawnSync } from "node:child_process";
import { accessSync, constants, mkdtempSync, rmSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createInterface } from "node:readline";

const WINDOWS = process.platform === "win32";
const MAX_INPUT = 200_000; // characters per text argument

// PATH's entries. On Windows, as libuv splits them: an entry that starts with a double or single
// quote runs to the matching quote and then to the next semicolon; one leading and one trailing
// quote are dropped.
function pathEntries() {
  const path = process.env.PATH ?? "";
  if (!WINDOWS) return path.split(":");
  return path.match(/(?:"[^"]*"?|'[^']*'?)?[^;]*/g).map((entry) =>
    entry.replace(/^["']/, "").replace(/["']$/, ""),
  );
}

// Only fully qualified entries are searched. Node on Windows looks in the working directory (the
// project the client runs in) first and a relative entry such as "." resolves there, so either
// could run a laconic-check the project planted; on Windows "\bin" depends on that directory's drive.
// On POSIX the name is appended without normalising, so "link/../bin" means what the system means.
function findBinary() {
  const qualified = WINDOWS ? /^(?:[A-Za-z]:[\\/]|[\\/]{2}[^\\/])/ : /^\//;
  const names = WINDOWS ? ["laconic-check.com", "laconic-check.exe"] : ["laconic-check"];
  for (const dir of pathEntries().filter((entry) => qualified.test(entry))) {
    for (const candidate of names.map((name) => (WINDOWS ? join(dir, name) : `${dir}/${name}`))) {
      try {
        if (statSync(candidate).isFile()) {
          accessSync(candidate, constants.X_OK);
          return candidate;
        }
      } catch {
        // missing or not executable: try the next one
      }
    }
  }
  throw new Error("could not find laconic-check in PATH's fully qualified entries; set LACONIC_CHECK to its full path");
}

const TOOLS = [
  {
    name: "lint",
    description:
      "Flag Australian English and house-style candidates in Markdown prose: em dashes, US spellings, 'while' and 'among', double quotation marks, possible serial commas, numbers 2 to 20 written as words, dates and times in non-Style Manual forms, weekdays that do not match their date, Latin abbreviations, chatbot closers, stock phrases and chat-tool residue. Code, URLs and quoted text are skipped. A finding is a reason to look, not proof of a defect.",
    inputSchema: {
      type: "object",
      properties: { text: { type: "string", description: "The Markdown or plain prose to check." } },
      required: ["text"],
    },
  },
  {
    name: "preserve",
    description:
      "Compare an original and an edited text and report numbers, sizes and units, dated months, inline code, URLs and meaning words and bounds (not, never, must, may, will, if, before, likely, some, all, only, unless, up to, about 50% and similar) that the edit dropped, then, as advice, those it added (an added figure or obligation can be invented) and any names to check. A clean result rules out these losses only; it does not prove the meaning survived.",
    inputSchema: {
      type: "object",
      properties: {
        original: { type: "string", description: "The text before editing." },
        edited: { type: "string", description: "The text after editing." },
      },
      required: ["original", "edited"],
    },
  },
];

function run(args, input) {
  const binary = process.env.LACONIC_CHECK || findBinary();
  const result = spawnSync(binary, args, { input, encoding: "utf8", timeout: 20_000, windowsHide: true });
  if (result.error) throw new Error(`could not run ${binary}: ${result.error.message}`);
  if (result.status === 2) throw new Error(result.stderr.trim() || "laconic-check failed");
  return result.stdout.trim();
}

function callTool(name, args) {
  for (const value of Object.values(args ?? {})) {
    if (typeof value !== "string") throw new Error("every argument must be a string");
    if (value.length > MAX_INPUT) throw new Error(`each text must be at most ${MAX_INPUT} characters`);
  }
  if (name === "lint") {
    if (typeof args?.text !== "string") throw new Error("text is required");
    const out = run(["lint", "--warn", "-"], args.text);
    return out ? out.replaceAll(/^-:/gm, "line ") : "No findings.";
  }
  if (name === "preserve") {
    if (typeof args?.original !== "string" || typeof args?.edited !== "string") throw new Error("original and edited are required");
    const dir = mkdtempSync(join(tmpdir(), "laconic-check-"));
    try {
      writeFileSync(join(dir, "original.md"), args.original, "utf8");
      writeFileSync(join(dir, "edited.md"), args.edited, "utf8");
      return run(["preserve", join(dir, "original.md"), join(dir, "edited.md")], "");
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  }
  throw new Error(`unknown tool: ${name}`);
}

function reply(id, result, error) {
  const message = error ? { jsonrpc: "2.0", id, error } : { jsonrpc: "2.0", id, result };
  process.stdout.write(JSON.stringify(message) + "\n");
}

const lines = createInterface({ input: process.stdin });
lines.on("line", (line) => {
  let request;
  try {
    request = JSON.parse(line);
  } catch {
    return reply(null, undefined, { code: -32700, message: "parse error" });
  }
  const { id, method, params } = request;
  if (id === undefined) return; // notifications need no reply
  switch (method) {
    case "initialize":
      return reply(id, {
        protocolVersion: params?.protocolVersion ?? "2025-06-18",
        capabilities: { tools: {} },
        serverInfo: { name: "laconic-check", version: "0.1.0" },
      });
    case "ping":
      return reply(id, {});
    case "tools/list":
      return reply(id, { tools: TOOLS });
    case "tools/call":
      try {
        return reply(id, { content: [{ type: "text", text: callTool(params?.name, params?.arguments) }] });
      } catch (e) {
        return reply(id, { content: [{ type: "text", text: e.message }], isError: true });
      }
    default:
      return reply(id, undefined, { code: -32601, message: `method not found: ${method}` });
  }
});

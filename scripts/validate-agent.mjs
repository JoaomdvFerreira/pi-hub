#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import process from "node:process";

const isWindows = process.platform === "win32";

function executable(name) {
  if (isWindows && (name === "npm" || name === "aiqt")) {
    return `${name}.cmd`;
  }
  return name;
}

const checks = [
  {
    name: "Rust check",
    command: "cargo",
    args: ["check", "--manifest-path", "src-tauri/Cargo.toml"],
  },
  {
    name: "Rust tests",
    command: "cargo",
    args: ["test", "--manifest-path", "src-tauri/Cargo.toml"],
  },
  {
    name: "Rust build",
    command: "cargo",
    args: ["build", "--manifest-path", "src-tauri/Cargo.toml"],
  },
  {
    name: "Frontend tests",
    command: "npm",
    args: ["test"],
  },
  {
    name: "Frontend build",
    command: "npm",
    args: ["run", "build"],
  },
  {
    name: "Frontend lint",
    command: "npm",
    args: ["run", "lint"],
    summarize: summarizeLint,
  },
  {
    name: "AIQT status",
    command: "aiqt",
    args: ["status"],
  },
  {
    name: "Diff check",
    command: "git",
    args: ["diff", "--check"],
  },
];

function normalizeOutput(result) {
  return `${result.stdout ?? ""}${result.stderr ?? ""}`.replace(/\r\n/g, "\n");
}

function summarizeLint(output) {
  const matches = [...output.matchAll(/(\d+)\s+warnings?\b/gi)];
  if (matches.length === 0) return "";
  const count = Number(matches.at(-1)[1]);
  return count > 0 ? `${count} warning${count === 1 ? "" : "s"}` : "";
}

function run(check) {
  const result = spawnSync(executable(check.command), check.args, {
    cwd: process.cwd(),
    encoding: "utf8",
    env: {
      ...process.env,
      CARGO_TERM_COLOR: "never",
      NO_COLOR: "1",
    },
    maxBuffer: 32 * 1024 * 1024,
    shell: isWindows && (check.command === "npm" || check.command === "aiqt"),
    windowsHide: true,
  });

  const output = normalizeOutput(result);

  if (result.error) {
    return {
      ok: false,
      detail: `${result.error.name}: ${result.error.message}`,
      output,
    };
  }

  if (result.status !== 0) {
    return {
      ok: false,
      detail: `exit ${result.status ?? "unknown"}`,
      output,
    };
  }

  const extra = check.summarize ? check.summarize(output) : "";
  return { ok: true, extra, output: "" };
}

console.log("Pi-Hub validation\n");

for (const check of checks) {
  const result = run(check);

  if (!result.ok) {
    console.error(`${check.name.padEnd(20)} FAIL — ${result.detail}`);
    if (result.output.trim()) {
      console.error("\n--- command output ---");
      console.error(result.output.trimEnd());
      console.error("--- end output ---");
    }
    process.exit(1);
  }

  const suffix = result.extra ? ` — ${result.extra}` : "";
  console.log(`${check.name.padEnd(20)} PASS${suffix}`);
}

console.log("\nAll canonical validation checks passed.");

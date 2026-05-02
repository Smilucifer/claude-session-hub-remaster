import { existsSync, readFileSync } from "node:fs";

const failures = [];

function requireFile(path) {
  if (!existsSync(path)) {
    failures.push(`Missing required file: ${path}`);
  }
}

function requireText(path, text) {
  if (!existsSync(path)) return;
  const content = readFileSync(path, "utf8");
  if (!content.includes(text)) {
    failures.push(`${path} does not mention required text: ${text}`);
  }
}

function checkScriptTargets() {
  const pkg = JSON.parse(readFileSync("package.json", "utf8"));
  for (const [name, script] of Object.entries(pkg.scripts ?? {})) {
    const matches = [...script.matchAll(/\bnode\s+(scripts\/[^\s;&|]+)/g)];
    for (const match of matches) {
      const target = match[1];
      if (!existsSync(target)) {
        failures.push(`package script "${name}" points to missing file: ${target}`);
      }
    }
  }
}

requireFile("README.md");
requireFile("README.zh-CN.md");

requireText("README.md", "Claude Session Hub Remaster");
requireText("README.md", "Rooms, Roundtable, and Driver/Copilot");
requireText("README.zh-CN.md", "Claude Session Hub Remaster");
requireText("README.zh-CN.md", "Rooms、Roundtable 和 Driver/Copilot");

checkScriptTargets();

if (failures.length > 0) {
  console.error("doc check failed:");
  for (const failure of failures) {
    console.error(`- ${failure}`);
  }
  process.exit(1);
}

console.log("doc check: ok");

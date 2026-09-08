#!/usr/bin/env node
/**
 * Regenerates THIRD_PARTY_NOTICES.md from cargo metadata and npm
 * production dependencies.
 *
 * Usage (from the repository root):
 *   node scripts/licenses-report.mjs
 */
import { spawnSync } from "node:child_process";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const outPath = join(root, "THIRD_PARTY_NOTICES.md");

const PDFIUM_NOTICE = `## PDFium

Abakus bundles a Windows x64 \`pdfium.dll\` from the
[bblanchon/pdfium-binaries](https://github.com/bblanchon/pdfium-binaries)
release tag \`chromium/7469\` (\`pdfium-win-x64.tgz\`). The development
fetch script is \`scripts/fetch-pdfium.ps1\`. The archive hash is pinned
in \`scripts/pdfium.sha256\`.

PDFium is developed by Google. The project license is BSD-3-Clause.
The text below is the BSD-3-Clause portion of the upstream PDFium
\`LICENSE\` file.

\`\`\`
Copyright 2014 The PDFium Authors

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are
met:

   * Redistributions of source code must retain the above copyright
notice, this list of conditions and the following disclaimer.
   * Redistributions in binary form must reproduce the above
copyright notice, this list of conditions and the following disclaimer
in the documentation and/or other materials provided with the
distribution.
   * Neither the name of Google Inc. nor the names of its
contributors may be used to endorse or promote products derived from
this software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
"AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT
OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
(INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
\`\`\`
`;

const NOTO_NOTICE = `## Noto Sans (PDF reports)

Abakus embeds \`NotoSans-Regular.ttf\` and \`NotoSans-Bold.ttf\`
(Noto Sans 2.008) in PDF reports. The files come from the archived
\`notofonts/noto-fonts\` repository. They are licensed under the SIL
Open Font License 1.1. The full license is
[\`src-tauri/assets/report-fonts/OFL.txt\`](src-tauri/assets/report-fonts/OFL.txt).
`;

function runCargoMetadata() {
  const result = spawnSync(
    "cargo",
    ["metadata", "--format-version", "1", "--offline", "--locked"],
    {
      cwd: root,
      encoding: "utf8",
      maxBuffer: 64 * 1024 * 1024,
    },
  );
  if (result.status !== 0) {
    const retry = spawnSync("cargo", ["metadata", "--format-version", "1"], {
      cwd: root,
      encoding: "utf8",
      maxBuffer: 64 * 1024 * 1024,
    });
    if (retry.status !== 0) {
      throw new Error(
        `cargo metadata failed:\n${retry.stderr || result.stderr}`,
      );
    }
    return JSON.parse(retry.stdout);
  }
  return JSON.parse(result.stdout);
}

function isNormalOrBuild(dep) {
  const kinds = dep.dep_kinds;
  if (!kinds || kinds.length === 0) {
    return true;
  }
  return kinds.some((k) => k.kind === null || k.kind === "build");
}

function collectRustPackages(metadata) {
  const byId = new Map(metadata.packages.map((p) => [p.id, p]));
  const nodes = new Map(metadata.resolve.nodes.map((n) => [n.id, n]));
  const workspace = new Set(metadata.workspace_members);
  const wanted = new Set();
  const stack = [...metadata.workspace_members];
  while (stack.length > 0) {
    const id = stack.pop();
    const node = nodes.get(id);
    if (!node) {
      continue;
    }
    for (const dep of node.deps) {
      if (!isNormalOrBuild(dep)) {
        continue;
      }
      if (wanted.has(dep.pkg) || workspace.has(dep.pkg)) {
        continue;
      }
      wanted.add(dep.pkg);
      stack.push(dep.pkg);
    }
  }
  const rows = [];
  for (const id of wanted) {
    const pkg = byId.get(id);
    if (!pkg) {
      continue;
    }
    rows.push({
      name: pkg.name,
      version: pkg.version,
      license: pkg.license || "(see crate)",
    });
  }
  rows.sort((a, b) => a.name.localeCompare(b.name) || a.version.localeCompare(b.version));
  return rows;
}

function lockKeyCandidates(name, fromKey) {
  const keys = [];
  if (fromKey) {
    keys.push(`${fromKey}/node_modules/${name}`);
    let cursor = fromKey;
    while (cursor.startsWith("node_modules/")) {
      const idx = cursor.lastIndexOf("/node_modules/");
      if (idx === -1) {
        break;
      }
      cursor = cursor.slice(0, idx);
      keys.push(`${cursor}/node_modules/${name}`);
    }
  }
  keys.push(`node_modules/${name}`);
  return keys;
}

function resolveLockKey(packages, name, fromKey) {
  for (const key of lockKeyCandidates(name, fromKey)) {
    if (packages[key]) {
      return key;
    }
  }
  return null;
}

function readNpmLicense(lockEntry, lockKey) {
  if (lockEntry.license) {
    return Array.isArray(lockEntry.license)
      ? lockEntry.license.join(" OR ")
      : String(lockEntry.license);
  }
  const pkgJsonPath = join(root, lockKey, "package.json");
  if (existsSync(pkgJsonPath)) {
    const pkg = JSON.parse(readFileSync(pkgJsonPath, "utf8"));
    if (pkg.license) {
      return typeof pkg.license === "string"
        ? pkg.license
        : JSON.stringify(pkg.license);
    }
    if (pkg.licenses) {
      return pkg.licenses
        .map((l) => (typeof l === "string" ? l : l.type))
        .filter(Boolean)
        .join(" OR ");
    }
  }
  return "(see package)";
}

function collectNpmPackages() {
  const lockPath = join(root, "package-lock.json");
  const pkgPath = join(root, "package.json");
  const lock = JSON.parse(readFileSync(lockPath, "utf8"));
  const pkg = JSON.parse(readFileSync(pkgPath, "utf8"));
  const packages = lock.packages || {};
  const direct = Object.keys(pkg.dependencies || {});
  const seen = new Set();
  const rows = [];

  function visit(name, fromKey) {
    const key = resolveLockKey(packages, name, fromKey);
    if (!key || seen.has(key)) {
      return;
    }
    const entry = packages[key];
    if (!entry || entry.dev) {
      return;
    }
    seen.add(key);
    rows.push({
      name,
      version: entry.version || "(unknown)",
      license: readNpmLicense(entry, key),
    });
    const deps = {
      ...(entry.dependencies || {}),
      ...(entry.optionalDependencies || {}),
    };
    for (const child of Object.keys(deps)) {
      visit(child, key);
    }
  }

  for (const name of direct) {
    visit(name, "");
  }
  rows.sort((a, b) => a.name.localeCompare(b.name) || a.version.localeCompare(b.version));
  return { direct, rows };
}

function mdCell(value) {
  return String(value).replaceAll("|", "\\|").replaceAll("\n", " ");
}

function table(rows) {
  const lines = [
    "| Name | Version | License |",
    "| --- | --- | --- |",
  ];
  for (const row of rows) {
    lines.push(
      `| ${mdCell(row.name)} | ${mdCell(row.version)} | ${mdCell(row.license)} |`,
    );
  }
  return lines.join("\n");
}

function main() {
  const metadata = runCargoMetadata();
  const rustRows = collectRustPackages(metadata);
  const npm = collectNpmPackages();

  const missingDirect = [];
  const rustNames = new Set(rustRows.map((r) => r.name));
  const cargoDirect = [
    "chrono",
    "serde",
    "serde_json",
    "thiserror",
    "regex",
    "sha2",
    "hex",
    "unicode-normalization",
    "pdfium-render",
    "strsim",
    "toml",
    "rusqlite",
    "tempfile",
    "tauri",
    "tauri-plugin-dialog",
    "tauri-build",
    "keyring",
    "netstat2",
    "reqwest",
    "lopdf",
    "ttf-parser",
  ];
  for (const name of cargoDirect) {
    if (!rustNames.has(name)) {
      missingDirect.push(`rust:${name}`);
    }
  }
  const npmNames = new Set(npm.rows.map((r) => r.name));
  for (const name of npm.direct) {
    if (!npmNames.has(name)) {
      missingDirect.push(`npm:${name}`);
    }
  }
  if (missingDirect.length > 0) {
    throw new Error(
      `missing direct production dependencies: ${missingDirect.join(", ")}`,
    );
  }

  const body = `# Third-party notices

This file is generated. Do not edit the tables by hand.

Generation command (repository root):

\`\`\`
node scripts/licenses-report.mjs
\`\`\`

The Rust table comes from \`cargo metadata --format-version 1\`. Workspace
members (\`abakus\`, \`parser\`, \`rules\`, \`store\`) are excluded. Only
normal and build dependencies of those members are listed, not
dev-dependencies.

The npm table lists production dependencies of \`package.json\`
(\`dependencies\`, not \`devDependencies\`) and their transitive
production packages from \`package-lock.json\`. License strings are
taken from the lockfile or from \`node_modules/*/package.json\`.

${PDFIUM_NOTICE}
${NOTO_NOTICE}
## Rust crates

${table(rustRows)}

## npm packages

${table(npm.rows)}
`;

  writeFileSync(outPath, body, "utf8");
  process.stdout.write(
    `Wrote ${outPath} (${rustRows.length} Rust crates, ${npm.rows.length} npm packages)\n`,
  );
}

main();

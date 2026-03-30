/// <reference types="node" />

import assert from "node:assert/strict";
import { readdir, readFile } from "node:fs/promises";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const ROOT = fileURLToPath(new URL(".", import.meta.url));

const ALLOWED_RAW_BUTTON_FILES = [
  "components/ui/notification-center.tsx",
  "features/tenants/tenant-usage-section.tsx",
  "pages/ImportJobs.tsx",
  "pages/Login.tsx",
] as const;

async function collectSourceFiles(directory: string): Promise<string[]> {
  const entries = await readdir(directory, { withFileTypes: true });
  const files = await Promise.all(
    entries.map(async (entry) => {
      const nextPath = path.join(directory, entry.name);
      if (entry.isDirectory()) {
        return collectSourceFiles(nextPath);
      }
      if (entry.isFile() && /\.(ts|tsx)$/.test(entry.name)) {
        return [nextPath];
      }
      return [];
    }),
  );

  return files.flat();
}

function toRelativePath(absolutePath: string) {
  return path.relative(ROOT, absolutePath).replaceAll(path.sep, "/");
}

test("frontend sources no longer import the deprecated button wrapper", async () => {
  const sourceFiles = await collectSourceFiles(ROOT);
  const offenders: string[] = [];

  for (const absolutePath of sourceFiles) {
    const source = await readFile(absolutePath, "utf8");
    if (/components\/ui\/button/.test(source)) {
      offenders.push(toRelativePath(absolutePath));
    }
  }

  assert.deepEqual(
    offenders,
    [],
    "All frontend surfaces should import HeroUI Button directly instead of the deprecated wrapper",
  );
});

test("manual raw button usage stays constrained to the audited exception list", async () => {
  const sourceFiles = await collectSourceFiles(ROOT);
  const allowedSet = new Set(ALLOWED_RAW_BUTTON_FILES);
  const rawButtonFiles: string[] = [];
  const unexpectedFiles: string[] = [];

  for (const absolutePath of sourceFiles) {
    if (/\.test\.tsx?$/.test(absolutePath)) {
      continue;
    }
    const source = await readFile(absolutePath, "utf8");
    if (!source.includes("<button")) {
      continue;
    }
    const relativePath = toRelativePath(absolutePath);
    rawButtonFiles.push(relativePath);
    if (!allowedSet.has(relativePath as (typeof ALLOWED_RAW_BUTTON_FILES)[number])) {
      unexpectedFiles.push(relativePath);
    }
  }

  assert.deepEqual(
    unexpectedFiles,
    [],
    "Raw <button> should only remain in audited exception files with non-CTA interaction semantics",
  );
  assert.deepEqual(
    rawButtonFiles.sort(),
    [...ALLOWED_RAW_BUTTON_FILES].sort(),
    "The manual button exception list should stay explicit and complete",
  );
});

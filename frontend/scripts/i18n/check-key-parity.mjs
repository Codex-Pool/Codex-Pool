#!/usr/bin/env node

import { readFileSync } from "node:fs";
import path from "node:path";
import process from "node:process";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";

const require = createRequire(import.meta.url);
const ts = require("typescript");

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const localesDir = path.resolve(__dirname, "../../src/locales");

const baselineLocale = "zh-CN";
const targetLocales = ["en"];

function getPropertyName(node) {
  if (ts.isIdentifier(node) || ts.isPrivateIdentifier(node)) {
    return node.text;
  }
  if (ts.isStringLiteral(node) || ts.isNumericLiteral(node) || ts.isNoSubstitutionTemplateLiteral(node)) {
    return node.text;
  }
  if (ts.isComputedPropertyName(node)) {
    const expression = node.expression;
    if (
      ts.isStringLiteral(expression) ||
      ts.isNumericLiteral(expression) ||
      ts.isNoSubstitutionTemplateLiteral(expression)
    ) {
      return expression.text;
    }
  }
  return null;
}

function collectLeafKeys(objectLiteral, prefix = "", output = new Set()) {
  for (const property of objectLiteral.properties) {
    if (ts.isSpreadAssignment(property)) {
      continue;
    }

    if (
      !ts.isPropertyAssignment(property) &&
      !ts.isShorthandPropertyAssignment(property) &&
      !ts.isMethodDeclaration(property)
    ) {
      continue;
    }

    const key = getPropertyName(property.name);
    if (!key) {
      continue;
    }

    const fullKey = prefix ? `${prefix}.${key}` : key;

    if (ts.isPropertyAssignment(property) && ts.isObjectLiteralExpression(property.initializer)) {
      collectLeafKeys(property.initializer, fullKey, output);
      continue;
    }

    output.add(fullKey);
  }

  return output;
}

function getDefaultExportObject(sourceFile) {
  for (const statement of sourceFile.statements) {
    if (ts.isExportAssignment(statement) && ts.isObjectLiteralExpression(statement.expression)) {
      return statement.expression;
    }
  }
  return null;
}

function loadLocaleKeys(locale) {
  const filePath = path.join(localesDir, `${locale}.ts`);
  const content = readFileSync(filePath, "utf8");
  const sourceFile = ts.createSourceFile(filePath, content, ts.ScriptTarget.Latest, true, ts.ScriptKind.TS);
  const objectLiteral = getDefaultExportObject(sourceFile);

  if (!objectLiteral) {
    throw new Error(`无法解析默认导出对象: ${filePath}`);
  }

  return collectLeafKeys(objectLiteral);
}

function compareLocaleKeys() {
  const baselineKeys = loadLocaleKeys(baselineLocale);
  let hasMismatch = false;

  for (const locale of targetLocales) {
    const localeKeys = loadLocaleKeys(locale);
    const missing = [...baselineKeys].filter((key) => !localeKeys.has(key)).sort();
    const extra = [...localeKeys].filter((key) => !baselineKeys.has(key)).sort();

    if (missing.length === 0 && extra.length === 0) {
      console.log(`[${locale}] key parity passed (${localeKeys.size} keys)`);
      continue;
    }

    hasMismatch = true;
    console.error(`[${locale}] key parity failed`);

    if (missing.length > 0) {
      console.error(`  Missing keys (${missing.length}):`);
      for (const key of missing) {
        console.error(`    - ${key}`);
      }
    }

    if (extra.length > 0) {
      console.error(`  Extra keys (${extra.length}):`);
      for (const key of extra) {
        console.error(`    - ${key}`);
      }
    }
  }

  if (hasMismatch) {
    process.exit(1);
  }

  console.log(
    `All locale files match baseline (${baselineLocale}.ts): ${targetLocales.join(", ")}`
  );
}

try {
  compareLocaleKeys();
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
}

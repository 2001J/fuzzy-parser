#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import { existsSync, statSync } from "node:fs";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const SCRIPT_PATH = fileURLToPath(import.meta.url);
const REPOSITORY_ROOT = path.resolve(path.dirname(SCRIPT_PATH), "../..");

function withoutFencedCode(markdown) {
  let fence = null;
  return markdown
    .split(/\r?\n/)
    .map((line) => {
      const marker = line.match(/^\s*(\x60{3,}|~{3,})/u)?.[1] ?? null;
      if (marker !== null) {
        if (fence === null) {
          fence = marker[0];
        } else if (marker[0] === fence) {
          fence = null;
        }
        return "";
      }
      return fence === null ? line : "";
    })
    .join("\n");
}

function slugifyHeading(value) {
  return value
    .replace(/\[([^\]]+)\]\([^)]*\)/gu, "$1")
    .replace(/<[^>]*>/gu, "")
    .replace(/[\x60*_~]/gu, "")
    .replace(/[^\p{L}\p{N}\s_-]/gu, "")
    .trim()
    .toLowerCase()
    .replace(/\s/gu, "-");
}

export function headingAnchors(markdown) {
  const anchors = new Set();
  const duplicateCounts = new Map();
  for (const line of withoutFencedCode(markdown).split("\n")) {
    const heading = line.match(/^\s{0,3}#{1,6}\s+(.+?)\s*#*\s*$/u);
    if (heading === null) {
      continue;
    }
    const base = slugifyHeading(heading[1]);
    if (base.length === 0) {
      continue;
    }
    const duplicate = duplicateCounts.get(base) ?? 0;
    anchors.add(duplicate === 0 ? base : base + "-" + duplicate);
    duplicateCounts.set(base, duplicate + 1);
  }
  return anchors;
}

export function localLinkTargets(markdown) {
  const targets = [];
  const source = withoutFencedCode(markdown);
  const link = /!?\[[^\]]*\]\((<[^>]+>|[^\s)]+)(?:\s+(?:"[^"]*"|'[^']*'))?\)/gu;
  for (const match of source.matchAll(link)) {
    const raw = match[1].replace(/^<|>$/gu, "");
    if (
      raw.startsWith("http://") ||
      raw.startsWith("https://") ||
      raw.startsWith("mailto:") ||
      raw.startsWith("data:")
    ) {
      continue;
    }
    targets.push(raw);
  }
  return targets;
}

function trackedMarkdownFiles(root) {
  const output = execFileSync(
    "git",
    ["ls-files", "--cached", "--others", "--exclude-standard", "--", "*.md"],
    { cwd: root, encoding: "utf8" },
  );
  return output
    .split(/\r?\n/u)
    .filter(Boolean)
    .map((relative) => path.resolve(root, relative));
}

function decode(value, source) {
  try {
    return decodeURIComponent(value);
  } catch {
    throw new Error(source + ": invalid URL encoding in link target " + JSON.stringify(value));
  }
}

export async function checkDocumentationLinks(root = REPOSITORY_ROOT) {
  const markdownFiles = trackedMarkdownFiles(root);
  const markdown = new Map();
  for (const file of markdownFiles) {
    markdown.set(file, await readFile(file, "utf8"));
  }

  const failures = [];
  for (const [sourcePath, source] of markdown) {
    const sourceName = path.relative(root, sourcePath);
    for (const rawTarget of localLinkTargets(source)) {
      const hashAt = rawTarget.indexOf("#");
      const rawFile = hashAt === -1 ? rawTarget : rawTarget.slice(0, hashAt);
      const rawAnchor = hashAt === -1 ? "" : rawTarget.slice(hashAt + 1);
      const cleanFile = rawFile.split("?")[0];
      let targetPath;
      try {
        targetPath =
          cleanFile.length === 0
            ? sourcePath
            : path.resolve(path.dirname(sourcePath), decode(cleanFile, sourceName));
      } catch (error) {
        failures.push(error.message);
        continue;
      }

      if (!existsSync(targetPath)) {
        failures.push(sourceName + ": missing local target " + rawTarget);
        continue;
      }
      if (rawAnchor.length === 0 || statSync(targetPath).isDirectory()) {
        continue;
      }
      if (path.extname(targetPath).toLowerCase() !== ".md") {
        failures.push(sourceName + ": anchor on non-Markdown target " + rawTarget);
        continue;
      }
      const targetSource = markdown.get(targetPath) ?? (await readFile(targetPath, "utf8"));
      const anchor = decode(rawAnchor, sourceName).toLowerCase();
      if (!headingAnchors(targetSource).has(anchor)) {
        failures.push(sourceName + ": missing anchor " + rawTarget);
      }
    }
  }

  if (failures.length > 0) {
    throw new Error(
      "documentation link check failed:\n" +
        failures.map((item) => "- " + item).join("\n"),
    );
  }
  return { files: markdownFiles.length };
}

if (path.resolve(process.argv[1] ?? "") === SCRIPT_PATH) {
  const result = await checkDocumentationLinks();
  process.stdout.write(
    "documentation links valid across " + result.files + " Markdown files\n",
  );
}

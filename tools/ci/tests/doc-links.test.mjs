import assert from "node:assert/strict";
import test from "node:test";

import { headingAnchors, localLinkTargets } from "../check-doc-links.mjs";

test("heading anchors follow GitHub-style punctuation and duplicate rules", () => {
  const markdown = [
    "# Contextual text/name migration (#13)",
    "## Resource limits — implemented",
    "## Same heading",
    "## Same heading",
    "",
    "~~~md",
    "# Not a heading",
    "~~~",
  ].join("\n");
  assert.deepEqual(
    [...headingAnchors(markdown)],
    [
      "contextual-textname-migration-13",
      "resource-limits--implemented",
      "same-heading",
      "same-heading-1",
    ],
  );
});

test("local link extraction ignores external links and fenced examples", () => {
  const markdown = [
    "[guide](docs/guide.md#start)",
    "[external](https://example.test/page)",
    "![asset](images/example.png)",
    "",
    "~~~md",
    "[not real](missing.md)",
    "~~~",
  ].join("\n");
  assert.deepEqual(localLinkTargets(markdown), [
    "docs/guide.md#start",
    "images/example.png",
  ]);
});

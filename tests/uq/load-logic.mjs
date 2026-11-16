// Loads the page's logic.js for Node tests. logic.js starts `.pragma library` and makes no Qt
// calls, so dropping the `.pragma` and `.import` lines leaves plain JavaScript. It runs as the
// body of a function and the top-level `function` and `var` names are the API it returns.
import { readFileSync } from "node:fs";

export function loadLogic(path) {
  const source = readFileSync(path, "utf8")
    .split("\n")
    .filter((line) => !/^\s*\.(pragma|import)\b/.test(line))
    .join("\n");
  const names = [...source.matchAll(/^(?:function\s+([A-Za-z_$][\w$]*)|var\s+([A-Za-z_$][\w$]*))/gm)].map((m) => m[1] ?? m[2]);
  return new Function(`${source}\nreturn { ${names.join(", ")} };`)();
}

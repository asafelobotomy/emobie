// A reader for the small TOML subset page.toml uses: comments, `key = value`, `[table]` and `[[array of tables]]`, strings
// with escapes, arrays (also over several lines) and inline tables. Anything else throws, so the page can't quietly grow
// past what this reads. (Node has no TOML parser.)

function parseValue(src, at) {
  const skip = () => {
    while (at < src.length && /[ \t\r\n]/.test(src[at])) at++;
  };
  const value = () => {
    skip();
    const c = src[at];
    if (c === '"') {
      let out = "";
      at++;
      while (src[at] !== '"') {
        if (at >= src.length) throw new Error("unterminated string");
        if (src[at] === "\\") {
          const e = src[at + 1];
          const map = { n: "\n", t: "\t", '"': '"', "\\": "\\" };
          if (!(e in map)) throw new Error(`unsupported escape \\${e}`);
          out += map[e];
          at += 2;
        } else out += src[at++];
      }
      at++;
      return out;
    }
    if (c === "'") {
      // A literal string: no escapes, ends at the next quote.
      const end = src.indexOf("'", at + 1);
      if (end < 0) throw new Error("unterminated literal string");
      const out = src.slice(at + 1, end);
      at = end + 1;
      return out;
    }
    if (c === "[") {
      at++;
      const list = [];
      for (;;) {
        skip();
        if (src[at] === "#") {
          while (at < src.length && src[at] !== "\n") at++;
          continue;
        }
        if (src[at] === "]") {
          at++;
          return list;
        }
        list.push(value());
        skip();
        if (src[at] === ",") at++;
      }
    }
    if (c === "{") {
      at++;
      const table = {};
      for (;;) {
        skip();
        if (src[at] === "}") {
          at++;
          return table;
        }
        const m = /^[A-Za-z0-9_-]+/.exec(src.slice(at));
        if (!m) throw new Error(`bad key at ${src.slice(at, at + 20)}`);
        at += m[0].length;
        skip();
        if (src[at++] !== "=") throw new Error("expected =");
        table[m[0]] = value();
        skip();
        if (src[at] === ",") at++;
      }
    }
    const m = /^(true|false|-?\d+)/.exec(src.slice(at));
    if (!m) throw new Error(`unsupported value at ${src.slice(at, at + 20)}`);
    at += m[0].length;
    return m[0] === "true" ? true : m[0] === "false" ? false : Number(m[0]);
  };
  const v = value();
  return [v, at];
}

export function parseToml(text) {
  const root = {};
  let table = root;
  let at = 0;
  const lineEnd = () => {
    const i = text.indexOf("\n", at);
    return i < 0 ? text.length : i;
  };
  while (at < text.length) {
    const end = lineEnd();
    const line = text.slice(at, end).trim();
    if (line === "" || line.startsWith("#")) {
      at = end + 1;
      continue;
    }
    let m = /^\[\[([A-Za-z0-9_.-]+)\]\]\s*(#.*)?$/.exec(line);
    if (m) {
      (root[m[1]] ??= []).push((table = {}));
      at = end + 1;
      continue;
    }
    m = /^\[([A-Za-z0-9_.-]+)\]\s*(#.*)?$/.exec(line);
    if (m) {
      table = root[m[1]] = {};
      at = end + 1;
      continue;
    }
    m = /^([A-Za-z0-9_-]+)\s*=\s*/.exec(text.slice(at, end));
    if (!m) throw new Error(`cannot read: ${line}`);
    const [value, next] = parseValue(text, at + m[0].length);
    table[m[1]] = value;
    // the rest of the line may only be a comment
    const rest = text.slice(next, text.indexOf("\n", next) < 0 ? text.length : text.indexOf("\n", next)).trim();
    if (rest !== "" && !rest.startsWith("#")) throw new Error(`unexpected after value: ${rest}`);
    at = text.indexOf("\n", next) < 0 ? text.length : text.indexOf("\n", next) + 1;
  }
  return root;
}

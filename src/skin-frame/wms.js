// Lenient .wms parser. It is "almost XML": WMP is case-insensitive, accepts
// unquoted attributes, stray '&' and unclosed tags, so DOMParser is no use.
// Returns nodes { tag, attrs, children, text } with the tag and attribute
// names in lowercase.

/** @typedef {{ tag: string, attrs: Record<string, string>, children: WmsNode[], text: string }} WmsNode */

const ENTITIES = { amp: "&", lt: "<", gt: ">", quot: '"', apos: "'" };

/** @param {string} s */
function entities(s) {
  if (!s.includes("&")) return s;
  return s.replace(/&(#x[0-9a-f]+|#\d+|amp|lt|gt|quot|apos);/gi, (_, e) => {
    const k = e.toLowerCase();
    if (k[0] === "#") return String.fromCodePoint(k[1] === "x" ? parseInt(k.slice(2), 16) : parseInt(k.slice(1), 10));
    return ENTITIES[k];
  });
}

/** @param {string} src @returns {WmsNode} */
export function parseWms(src) {
  /** @type {WmsNode} */
  const root = { tag: "#root", attrs: {}, children: [], text: "" };
  const stack = [root];
  const n = src.length;
  let i = 0;
  const top = () => stack[stack.length - 1];

  while (i < n) {
    const lt = src.indexOf("<", i);
    if (lt < 0) break;
    if (lt > i) top().text += src.slice(i, lt);
    i = lt;
    if (src.startsWith("<!--", i)) {
      const end = src.indexOf("-->", i + 4);
      i = end < 0 ? n : end + 3;
      continue;
    }
    if (src.startsWith("<![CDATA[", i)) {
      const end = src.indexOf("]]>", i + 9);
      top().text += src.slice(i + 9, end < 0 ? n : end);
      i = end < 0 ? n : end + 3;
      continue;
    }
    if (src[i + 1] === "?" || src[i + 1] === "!") {
      const end = src.indexOf(">", i);
      i = end < 0 ? n : end + 1;
      continue;
    }
    if (src[i + 1] === "/") {
      const end = src.indexOf(">", i);
      const name = src.slice(i + 2, end < 0 ? n : end).trim().toLowerCase();
      i = end < 0 ? n : end + 1;
      // Closes up to the matching tag; a stray closing tag is ignored.
      for (let k = stack.length - 1; k > 0; k--) {
        if (stack[k].tag === name) {
          stack.length = k;
          break;
        }
      }
      continue;
    }
    const m = /^<([A-Za-z_][\w:.-]*)/.exec(src.slice(i, i + 64));
    if (!m) {
      top().text += "<";
      i++;
      continue;
    }
    /** @type {WmsNode} */
    const node = { tag: m[1].toLowerCase(), attrs: {}, children: [], text: "" };
    i += m[0].length;
    let selfClose = false;
    // Attributes.
    while (i < n) {
      while (i < n && /\s/.test(src[i])) i++;
      if (src[i] === ">") {
        i++;
        break;
      }
      if (src[i] === "/" && src[i + 1] === ">") {
        selfClose = true;
        i += 2;
        break;
      }
      if (src[i] === "<") break; // unclosed tag: left for the next iteration
      const nm = /^[^\s=/>]+/.exec(src.slice(i, i + 128));
      if (!nm) {
        i++;
        continue;
      }
      const name = nm[0].toLowerCase();
      i += nm[0].length;
      while (i < n && /\s/.test(src[i])) i++;
      let value = "";
      if (src[i] === "=") {
        i++;
        while (i < n && /\s/.test(src[i])) i++;
        const q = src[i];
        if (q === '"' || q === "'") {
          const end = src.indexOf(q, i + 1);
          value = src.slice(i + 1, end < 0 ? n : end);
          i = end < 0 ? n : end + 1;
        } else {
          const um = /^[^\s>]*/.exec(src.slice(i, i + 4096));
          value = um ? um[0] : "";
          if (value.endsWith("/") && src[i + value.length] === ">") value = value.slice(0, -1);
          i += value.length;
        }
      }
      if (!(name in node.attrs)) node.attrs[name] = entities(value);
    }
    top().children.push(node);
    if (selfClose) continue;
    if (node.tag === "script") {
      // Literal content up to </script>.
      const end = src.toLowerCase().indexOf("</script", i);
      node.text = src.slice(i, end < 0 ? n : end);
      const close = end < 0 ? n : src.indexOf(">", end);
      i = close < 0 ? n : close + 1;
      continue;
    }
    stack.push(node);
  }
  return root;
}

/** First node with that tag (depth-first). @param {WmsNode} node @param {string} tag @returns {WmsNode | null} */
export function findTag(node, tag) {
  for (const c of node.children) {
    if (c.tag === tag) return c;
    const f = findTag(c, tag);
    if (f) return f;
  }
  return null;
}

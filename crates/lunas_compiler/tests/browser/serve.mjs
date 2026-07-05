// serve.mjs — a dependency-free static file server for the browser smoke test.
//
// Usage: node serve.mjs <rootDir> <port>
// Serves files under <rootDir> with correct MIME for .mjs/.js/.html so a real
// browser can load the compiled ES modules + runtime. Prints "READY <port>" to
// stdout once listening (the Rust harness waits for that line), then serves
// until killed.

import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { extname, join, normalize } from "node:path";

const rootDir = process.argv[2];
const port = Number(process.argv[3] || 0);

const MIME = {
  ".mjs": "text/javascript; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".html": "text/html; charset=utf-8",
  ".json": "application/json; charset=utf-8",
};

const server = createServer(async (req, res) => {
  try {
    let path = decodeURIComponent(new URL(req.url, "http://x").pathname);
    if (path === "/") path = "/index.html";
    // Contain the path within rootDir (no traversal).
    const full = normalize(join(rootDir, path));
    if (!full.startsWith(normalize(rootDir))) {
      res.writeHead(403).end("forbidden");
      return;
    }
    const body = await readFile(full);
    const type = MIME[extname(full)] || "application/octet-stream";
    res.writeHead(200, { "content-type": type }).end(body);
  } catch {
    res.writeHead(404).end("not found");
  }
});

server.listen(port, "127.0.0.1", () => {
  const actual = server.address().port;
  process.stdout.write("READY " + actual + "\n");
});

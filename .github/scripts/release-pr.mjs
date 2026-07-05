#!/usr/bin/env node
// Squash-merge-aware release-PR generator (replaces the git-pr-release gem).
//
// Determines which PRs are contained in `main` but not `beta`, then creates or
// updates the open release PR (base=beta, head=main) with a categorized,
// checkbox-preserving body.
//
// Detection strategy
// ------------------
// git-pr-release detected released PRs ONLY via `Merge pull request #N` merge
// commits. This repo squash-merges, so those commits never exist. Instead we:
//   1. `GET /compare/beta...main` to list the commits in main-not-beta.
//   2. For each commit, first try to extract the PR number from the subject
//      (`... (#N)` for squash commits, `Merge pull request #N` for merges) —
//      this avoids an API round-trip for the common case.
//   3. For commits without a recognizable subject, fall back to
//      `GET /commits/{sha}/pulls`, which reports the associated PR(s) for both
//      squash and merge commits.
//   4. Dedupe by PR number and fetch each PR's metadata (labels, author).
//
// Dependency-free: Node >= 20, global fetch, GITHUB_TOKEN env.
//
// Usage:
//   node .github/scripts/release-pr.mjs            # create/update the release PR
//   node .github/scripts/release-pr.mjs --dry-run  # print body, no writes

const PRODUCTION_BRANCH = "beta";
const STAGING_BRANCH = "main";
const RELEASE_LABEL = "beta";

// Category sections, in output order. Each has a heading and the label that
// selects PRs into it. `others` is the catch-all for PRs matching no category.
const CATEGORIES = [
  { key: "fix", label: "fix", heading: "## 🛠 Fixes" },
  { key: "feature", label: "feature", heading: "## ✨ New Features" },
  { key: "refactor", label: "refactor", heading: "## 🔨 Refactoring" },
  { key: "version", label: "version", heading: "## 📌 Version Updates" },
  { key: "chore", label: "chore", heading: "## 🔧 Chore" },
];
const CATEGORY_LABELS = CATEGORIES.map((c) => c.label);
const OTHERS_HEADING = "## 👻 Others";

// ---------------------------------------------------------------------------
// Pure, unit-testable body generation
// ---------------------------------------------------------------------------

/**
 * Today's date in Asia/Tokyo as YYYY-MM-DD.
 * @param {Date} [now]
 */
export function tokyoDate(now = new Date()) {
  // en-CA formats as YYYY-MM-DD.
  return new Intl.DateTimeFormat("en-CA", {
    timeZone: "Asia/Tokyo",
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).format(now);
}

/**
 * Parse the set of already-checked PR numbers from an existing PR body.
 * Matches `- [x] #123` (case-insensitive on the x).
 * @param {string} body
 * @returns {Set<number>}
 */
export function parseCheckedNumbers(body) {
  const checked = new Set();
  if (!body) return checked;
  const re = /^- \[x\] #(\d+)\b/gim;
  let m;
  while ((m = re.exec(body)) !== null) {
    checked.add(Number(m[1]));
  }
  return checked;
}

/**
 * Build the release PR body from a list of PRs.
 *
 * @param {Array<{number: number, login: string, labels: string[]}>} pulls
 * @param {object} [opts]
 * @param {string} [opts.date]            YYYY-MM-DD (defaults to Tokyo today)
 * @param {Set<number>} [opts.checked]    PR numbers to render checked
 * @returns {string}
 */
export function buildBody(pulls, opts = {}) {
  const date = opts.date ?? tokyoDate();
  const checked = opts.checked ?? new Set();

  // Sort ascending by PR number for stable output.
  const sorted = [...pulls].sort((a, b) => a.number - b.number);

  const line = (pr) =>
    `- [${checked.has(pr.number) ? "x" : " "}] #${pr.number} @${pr.login}`;

  const out = [`🚀 Release (${date} - ${PRODUCTION_BRANCH})`];

  for (const cat of CATEGORIES) {
    out.push(cat.heading);
    for (const pr of sorted) {
      if (pr.labels.includes(cat.label)) out.push(line(pr));
    }
  }

  out.push(OTHERS_HEADING);
  for (const pr of sorted) {
    if (!pr.labels.some((l) => CATEGORY_LABELS.includes(l))) out.push(line(pr));
  }

  // Trailing newline to match the ERB template output.
  return out.join("\n") + "\n";
}

/**
 * Extract a PR number from a commit subject.
 * Handles squash subjects (`... (#123)`) and merge subjects
 * (`Merge pull request #123 from ...`).
 * @param {string} subject
 * @returns {number|null}
 */
export function prNumberFromSubject(subject) {
  if (!subject) return null;
  const merge = subject.match(/^Merge pull request #(\d+)\b/);
  if (merge) return Number(merge[1]);
  const squash = subject.match(/\(#(\d+)\)\s*$/);
  if (squash) return Number(squash[1]);
  return null;
}

// ---------------------------------------------------------------------------
// GitHub REST helpers
// ---------------------------------------------------------------------------

function requireEnv(name) {
  const v = process.env[name];
  if (!v) {
    console.error(`Missing required environment variable: ${name}`);
    process.exit(1);
  }
  return v;
}

class GitHub {
  constructor(token, repo) {
    this.token = token;
    this.repo = repo; // "owner/name"
    this.base = "https://api.github.com";
  }

  async request(method, path, body) {
    const url = path.startsWith("http") ? path : `${this.base}${path}`;
    const res = await fetch(url, {
      method,
      headers: {
        Authorization: `Bearer ${this.token}`,
        Accept: "application/vnd.github+json",
        "X-GitHub-Api-Version": "2022-11-28",
        "User-Agent": "lunas-release-pr-script",
        ...(body ? { "Content-Type": "application/json" } : {}),
      },
      body: body ? JSON.stringify(body) : undefined,
    });
    if (!res.ok) {
      const text = await res.text().catch(() => "");
      throw new Error(
        `GitHub API ${method} ${url} failed: ${res.status} ${res.statusText}\n${text}`,
      );
    }
    return res;
  }

  async json(method, path, body) {
    const res = await this.request(method, path, body);
    return res.json();
  }

  /**
   * Follow RFC 5988 `Link: rel="next"` pagination, yielding each parsed page.
   */
  async *paginate(path) {
    let next = `${this.base}${path}`;
    while (next) {
      const res = await this.request("GET", next);
      yield await res.json();
      next = parseNextLink(res.headers.get("link"));
    }
  }
}

function parseNextLink(linkHeader) {
  if (!linkHeader) return null;
  for (const part of linkHeader.split(",")) {
    const m = part.match(/<([^>]+)>;\s*rel="next"/);
    if (m) return m[1];
  }
  return null;
}

// ---------------------------------------------------------------------------
// Detection: PRs in main but not beta
// ---------------------------------------------------------------------------

/**
 * Collect all commits in `beta...main` via the compare API, paginating.
 * Returns an array of { sha, subject }.
 */
async function compareCommits(gh) {
  const commits = [];
  const path = `/repos/${gh.repo}/compare/${PRODUCTION_BRANCH}...${STAGING_BRANCH}?per_page=100`;
  for await (const page of gh.paginate(path)) {
    for (const c of page.commits ?? []) {
      const subject = (c.commit?.message ?? "").split("\n", 1)[0];
      commits.push({ sha: c.sha, subject });
    }
  }
  return commits;
}

/**
 * Given compare commits, resolve the set of PR numbers.
 * Subject-first (no API call), falling back to /commits/{sha}/pulls.
 */
async function resolvePrNumbers(gh, commits) {
  const numbers = new Set();
  for (const c of commits) {
    const fromSubject = prNumberFromSubject(c.subject);
    if (fromSubject !== null) {
      numbers.add(fromSubject);
      continue;
    }
    // Fallback: ask GitHub which PR(s) this commit belongs to.
    const pulls = await gh.json(
      "GET",
      `/repos/${gh.repo}/commits/${c.sha}/pulls?per_page=100`,
    );
    for (const p of pulls) {
      // Only count PRs that merged into the staging branch.
      if (p.number) numbers.add(p.number);
    }
  }
  return numbers;
}

/**
 * Fetch metadata (author login, label names) for each PR number.
 */
async function fetchPullDetails(gh, numbers) {
  const pulls = [];
  for (const number of numbers) {
    const pr = await gh.json("GET", `/repos/${gh.repo}/pulls/${number}`);
    pulls.push({
      number: pr.number,
      login: pr.user?.login ?? "unknown",
      labels: (pr.labels ?? []).map((l) => l.name),
    });
  }
  return pulls;
}

/**
 * Find the single open release PR (base=beta, head=main), if any.
 */
async function findReleasePr(gh) {
  const owner = gh.repo.split("/")[0];
  const list = await gh.json(
    "GET",
    `/repos/${gh.repo}/pulls?state=open&base=${PRODUCTION_BRANCH}&head=${owner}:${STAGING_BRANCH}&per_page=100`,
  );
  return list[0] ?? null;
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

async function main() {
  const dryRun = process.argv.includes("--dry-run");
  const token = requireEnv("GITHUB_TOKEN");
  const repo = requireEnv("GITHUB_REPOSITORY");
  const gh = new GitHub(token, repo);

  console.log(`Comparing ${PRODUCTION_BRANCH}...${STAGING_BRANCH} in ${repo}`);
  const commits = await compareCommits(gh);
  console.log(`Found ${commits.length} commit(s) ahead of ${PRODUCTION_BRANCH}.`);

  const numbers = await resolvePrNumbers(gh, commits);
  console.log(`Resolved ${numbers.size} unique PR(s).`);

  if (numbers.size === 0) {
    console.log("No PRs in beta..main — nothing to release. Exiting cleanly.");
    return;
  }

  const pulls = await fetchPullDetails(gh, numbers);

  const existing = await findReleasePr(gh);
  const checked = parseCheckedNumbers(existing?.body ?? "");
  const body = buildBody(pulls, { checked });
  const title = body.split("\n", 1)[0];

  if (dryRun) {
    console.log("\n--- DRY RUN: generated PR body ---\n");
    console.log(body);
    console.log("--- end DRY RUN ---");
    return;
  }

  if (existing) {
    console.log(`Updating existing release PR #${existing.number}.`);
    await gh.json("PATCH", `/repos/${gh.repo}/pulls/${existing.number}`, {
      title,
      body,
    });
    console.log(`Updated ${existing.html_url}`);
  } else {
    console.log("Creating new release PR.");
    const created = await gh.json("POST", `/repos/${gh.repo}/pulls`, {
      title,
      body,
      base: PRODUCTION_BRANCH,
      head: STAGING_BRANCH,
    });
    // Add the release label (best effort — labeling is separate from creation).
    await gh.json("POST", `/repos/${gh.repo}/issues/${created.number}/labels`, {
      labels: [RELEASE_LABEL],
    });
    console.log(`Created ${created.html_url}`);
  }
}

// Only run main when executed directly (not when imported by tests).
const isMain =
  import.meta.url === `file://${process.argv[1]}` ||
  process.argv[1]?.endsWith("release-pr.mjs");

if (isMain) {
  main().catch((err) => {
    console.error(err.message ?? err);
    process.exit(1);
  });
}

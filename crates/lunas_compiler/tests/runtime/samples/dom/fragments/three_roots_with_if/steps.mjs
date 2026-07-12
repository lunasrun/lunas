export default async ({ click, expect }) => {
  // The fragment (ThreeRoots) is mounted as a CHILD of a single-root App, so
  // the assertion kit's `$$` walks the wrapper div's live childNodes rather
  // than a frozen top-level `roots` array snapshot -- if the fragment were the
  // mounted App itself, a top-level root removed by :if would still be found
  // by `$$`/`expect(...).count()`, since the harness's `roots` array for a
  // fragment is captured once at construction and never re-synced with the
  // live DOM (a kit limitation, not a compiler/runtime bug).
  expect("h1").text("T");
  expect("p").count(1);
  await click("h1");
  expect("h1").text("T2");
  expect("p").count(0);
};

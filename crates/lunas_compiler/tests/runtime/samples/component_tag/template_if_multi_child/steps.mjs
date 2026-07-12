export default async ({ $$, click, expect, equal }) => {
  // Both spans render as siblings; NO literal <template> element in the DOM.
  equal($$("template").length, 0, "no literal <template> element survives");
  equal($$("span").length, 2, "both branch children mount as siblings");
  await click("button");
  equal($$("span").length, 0, "branch removed when condition false");
  await click("button");
  equal($$("span").length, 2, "branch re-mounts both children");
};

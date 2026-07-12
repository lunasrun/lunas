export default async ({ $, $$, click, expect, equal }) => {
  // Shown initially: the child renders with its prop.
  equal($$("span").length, 1, "child mounted when :if is true");
  expect($("span")).text("hi");
  // Toggle off: the child unmounts.
  await click("button");
  equal($$("span").length, 0, "child unmounted when :if goes false");
  // Toggle on: the child remounts fresh.
  await click("button");
  equal($$("span").length, 1, "child remounts when :if is true again");
  expect($("span")).text("hi");
};

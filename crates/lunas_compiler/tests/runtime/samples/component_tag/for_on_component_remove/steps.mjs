export default async ({ $$, click, expect, equal }) => {
  equal($$("li").length, 3, "three children");
  await click("button");
  const lis = $$("li");
  equal(lis.length, 2, "removed item's child unmounted");
  expect(lis[0]).text("a");
  expect(lis[1]).text("c");
};

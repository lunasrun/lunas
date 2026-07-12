export default async ({ $$, click, expect, equal }) => {
  // One child instance per item, props from the item.
  let lis = $$("li");
  equal(lis.length, 2, "one child per item");
  expect(lis[0]).text("a");
  expect(lis[1]).text("b");
  // Push a new item: a new child mounts (keyed diff, no re-render of others).
  await click("button");
  lis = $$("li");
  equal(lis.length, 3, "new item mounts a new child");
  expect(lis[2]).text("z");
};

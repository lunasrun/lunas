export default async ({ $$, click, expect, equal }) => {
  let lis = $$("li");
  expect(lis[0]).text("n=10");
  expect(lis[1]).text("n=20");
  // Mutate item 0's data: same keyed instance, prop updates via patch->runScope.
  await click("button");
  lis = $$("li");
  equal(lis.length, 2, "no remount");
  expect(lis[0]).text("n=99");
  expect(lis[1]).text("n=20");
};

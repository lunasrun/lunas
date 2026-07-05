export default async ({ $, $$, click, expect, equal }) => {
  expect("b").text("n=1");
  await click(".t");
  equal($$("section").length, 0);
  equal($$("b").length, 0);
  // mutating parent state after teardown must not resurrect content
  await click(".i");
  equal($$("section").length, 0);
};

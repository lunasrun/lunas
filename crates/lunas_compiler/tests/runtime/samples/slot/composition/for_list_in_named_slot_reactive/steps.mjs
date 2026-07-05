export default async ({ $$, click, equal }) => {
  equal($$(".it").length, 2);
  await click("button");
  equal($$(".it").length, 3);
};

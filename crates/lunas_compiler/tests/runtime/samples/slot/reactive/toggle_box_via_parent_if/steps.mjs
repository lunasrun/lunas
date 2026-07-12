export default async ({ $, $$, click, equal }) => {
  equal($$("section").length, 1);
  await click("button");
  equal($$("section").length, 0);
  await click("button");
  equal($$("section").length, 1);
};

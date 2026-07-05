export default async ({ $$, click, expect }) => {
  const spans = $$("span");
  expect(spans[0]).text("a");
  await click("button");
  expect($$("span")[0]).text("A");
};

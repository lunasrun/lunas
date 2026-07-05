export default async ({ $$, click, expect }) => {
  const spans = $$("span");
  expect(spans[0]).text("0");
  expect(spans[1]).text("0");
  await click("button");
  expect(spans[0]).text("1");
  expect(spans[1]).text("1");
};

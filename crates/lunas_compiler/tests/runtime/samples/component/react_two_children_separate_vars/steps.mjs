export default async ({ $$, click, expect }) => {
  const spans = $$("span");
  expect(spans[0]).text("0");
  expect(spans[1]).text("10");
  await click(".a");
  expect(spans[0]).text("1");
  expect(spans[1]).text("10");
  await click(".b");
  expect(spans[0]).text("1");
  expect(spans[1]).text("11");
};

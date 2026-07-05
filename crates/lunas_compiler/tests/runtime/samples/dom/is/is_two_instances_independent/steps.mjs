export default async ({ $$, click, expect }) => {
  const [incA] = $$(".inc");
  await click(incA);
  await click(incA);
  const spans = $$("span");
  expect(spans[0]).text("2");
  expect(spans[1]).text("0");
};

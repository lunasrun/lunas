export default async ({ $$, click, expect }) => {
  expect("span").count(1);
  await click("button");
  expect("span").count(2);
  const spans = $$("span");
  expect(spans[1]).text("x");
};

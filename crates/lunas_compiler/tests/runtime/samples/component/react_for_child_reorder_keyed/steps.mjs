export default async ({ $$, click, expect }) => {
  let spans = $$("span");
  expect(spans[0]).text("a");
  await click("button");
  spans = $$("span");
  expect(spans[0]).text("c");
  expect(spans[2]).text("a");
};

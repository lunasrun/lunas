export default async ({ $$, click, expect }) => {
  const items = $$(".item");
  expect(items[0]).attr("class", "item selected");
  expect(items[1]).attr("class", "item");
  await click(items[1]);
  expect(items[0]).attr("class", "item");
  expect(items[1]).attr("class", "item selected");
};

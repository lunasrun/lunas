export default async ({ $$, click, expect }) => {
  const rows = $$(".row");
  expect(rows[0]).attr("style", "color: red;");
  expect(rows[1]).attr("style", "color: blue;");
  await click(rows[0]);
  expect(rows[0]).attr("style", "color: green;");
};

export default async ({ $$, click, expect }) => {
  const rows = $$(".row");
  expect(rows[0]).attr("class", "row");
  expect(rows[1]).attr("class", "row done");
  await click(rows[0]);
  expect(rows[0]).attr("class", "row done");
};

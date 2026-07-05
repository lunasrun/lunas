export default async ({ $, $$, click, expect }) => {
  const span = $("span");
  const [pbtn, cbtn] = $$("button");
  expect(span).text("10");
  await click(pbtn);
  expect(span).text("11");
  await click(cbtn);
  expect(span).text("16");
};

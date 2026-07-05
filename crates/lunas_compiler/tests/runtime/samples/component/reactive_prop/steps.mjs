export default async ({ $, $$, click, tick, expect }) => {
  const span = $("span");
  const [pBtn, cBtn] = $$("button");
  expect(span).text("1+0");
  await click(pBtn);
  expect(span).text("2+0");
  await click(cBtn);
  expect(span).text("2+1");
  await click(pBtn);
  expect(span).text("3+1");
};

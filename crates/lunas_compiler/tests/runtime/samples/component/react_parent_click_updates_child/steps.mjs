export default async ({ $, click, expect }) => {
  const span = $("span");
  expect(span).text("1");
  await click("button");
  expect(span).text("2");
  await click("button");
  expect(span).text("3");
};

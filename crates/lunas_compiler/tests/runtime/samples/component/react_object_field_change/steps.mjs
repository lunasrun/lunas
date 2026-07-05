export default async ({ $, click, expect }) => {
  const span = $("span");
  expect(span).text("ada");
  await click("button");
  expect(span).text("grace");
};

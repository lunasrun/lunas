export default async ({ $, click, expect }) => {
  const span = $("span.base");
  await click("button");
  expect(span).hasClass("active");
  await click("button");
  expect(span).attr("class", "base");
};

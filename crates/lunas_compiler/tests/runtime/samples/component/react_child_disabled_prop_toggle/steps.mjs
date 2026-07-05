export default async ({ $, click, expect }) => {
  const c = $("button.c");
  expect(c).attr("class", "c");
  await click(".p");
  expect(c).hasClass("off");
  await click(".p");
  expect(c).attr("class", "c");
};

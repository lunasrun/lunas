export default async ({ $, click, expect }) => {
  const el = $("button.card");
  expect(el).hasClass("featured");
  await click(el);
  expect(el).hasClass("open");
  expect(el).hasClass("featured");
};

export default async ({ $, click, expect }) => {
  await click(".btn-a");
  await click(".btn-b");
  expect(".btn-a").text("1");
  expect(".btn-b").text("10");
};

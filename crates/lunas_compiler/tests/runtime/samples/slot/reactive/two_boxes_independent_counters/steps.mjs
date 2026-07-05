export default async ({ $, click, expect }) => {
  expect(".x").text("0");
  expect(".y").text("0");
  await click(".a");
  expect(".x").text("1");
  expect(".y").text("0");
  await click(".b");
  await click(".b");
  expect(".x").text("1");
  expect(".y").text("2");
};

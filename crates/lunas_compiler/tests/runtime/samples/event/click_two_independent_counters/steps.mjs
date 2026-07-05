export default async ({ $, click, expect }) => {
  await click(".a");
  await click(".a");
  await click(".b");
  expect(".a").text("a=2");
  expect(".b").text("b=1");
};

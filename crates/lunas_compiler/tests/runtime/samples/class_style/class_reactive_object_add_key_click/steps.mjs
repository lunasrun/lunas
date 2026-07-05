export default async ({ $, click, expect }) => {
  expect(".box").attr("class", "box a");
  await click(".box");
  expect(".box").attr("class", "box a b");
};

export default async ({ $, click, tick, expect }) => {
  expect(".box").attr("style", "color: red;");
  await click(".box");
  expect(".box").attr("style", "color: blue;");
};

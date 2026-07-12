export default async ({ $, click, expect }) => {
  expect(".box").attr("class", "box");
  expect(".box").attr("style", "color: gray;");
  await click(".box");
  expect(".box").attr("class", "box active");
  expect(".box").attr("style", "color: red;");
};

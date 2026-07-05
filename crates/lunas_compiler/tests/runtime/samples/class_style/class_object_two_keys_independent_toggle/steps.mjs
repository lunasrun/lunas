export default async ({ $, click, expect }) => {
  expect(".box").attr("class", "box");
  await click(".box");
  expect(".box").attr("class", "box active");
  await click("button");
  expect(".box").attr("class", "box active error");
};

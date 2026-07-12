export default async ({ click, expect }) => {
  expect(".box").attr("style", "color: red;");
  await click(".box");
  expect(".box").attr("style", "color: blue;");
};

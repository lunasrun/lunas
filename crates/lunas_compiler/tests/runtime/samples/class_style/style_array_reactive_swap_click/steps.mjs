export default async ({ $, click, expect }) => {
  expect(".tgt").attr("style", "color: orange;");
  await click(".tgt");
  expect(".tgt").attr("style", "color: cyan;");
};

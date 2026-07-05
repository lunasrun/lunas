export default async ({ $, click, expect }) => {
  expect(".tgt").attr("style", "color: red;");
  await click(".tgt");
  expect(".tgt").attr("style", "color: red; border: 1px solid black;");
  await click(".tgt");
  expect(".tgt").attr("style", "color: red;");
};

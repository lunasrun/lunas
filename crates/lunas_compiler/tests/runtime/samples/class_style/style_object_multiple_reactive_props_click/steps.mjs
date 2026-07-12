export default async ({ $, click, expect }) => {
  expect(".tgt").attr("style", "color: red; font-size: 10px;");
  await click(".tgt");
  expect(".tgt").attr("style", "color: green; font-size: 20px;");
};

export default async ({ $, click, expect }) => {
  expect(".tgt").attr("style", "color: red;");
  await click(".tgt");
  expect(".tgt").attr("style", "color: blue; font-weight: bold;");
};

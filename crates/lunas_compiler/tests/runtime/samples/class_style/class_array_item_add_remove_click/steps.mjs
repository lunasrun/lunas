export default async ({ $, click, expect }) => {
  expect(".tgt").attr("class", "tgt a b");
  await click(".rm");
  expect(".tgt").attr("class", "tgt a");
  await click(".add");
  expect(".tgt").attr("class", "tgt a c");
};

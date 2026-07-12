export default async ({ $, click, expect }) => {
  expect(".tgt").attr("class", "tgt a b");
  await click(".tgt");
  expect(".tgt").attr("class", "tgt c");
};

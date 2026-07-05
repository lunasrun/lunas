export default async ({ $, click, expect }) => {
  expect(".tgt").attr("class", "tgt a");
  await click(".tgt");
  expect(".tgt").attr("class", "tgt plain-string");
};

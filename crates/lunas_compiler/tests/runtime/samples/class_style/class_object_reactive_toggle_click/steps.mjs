export default async ({ $, click, expect }) => {
  expect(".tgt").attr("class", "tgt");
  await click(".tgt");
  expect(".tgt").hasClass("active");
  await click(".tgt");
  expect(".tgt").attr("class", "tgt");
};

export default async ({ $, click, expect }) => {
  expect(".one").attr("class", "one");
  expect(".two").attr("class", "two on");
  await click(".one");
  expect(".one").attr("class", "one on");
  expect(".two").attr("class", "two on");
  await click(".two");
  expect(".one").attr("class", "one on");
  expect(".two").attr("class", "two");
};

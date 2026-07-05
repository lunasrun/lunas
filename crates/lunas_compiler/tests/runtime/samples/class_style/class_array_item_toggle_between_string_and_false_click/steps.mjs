export default async ({ $, click, expect }) => {
  expect(".base").attr("class", "base visible");
  await click(".base");
  expect(".base").attr("class", "base");
  await click(".base");
  expect(".base").attr("class", "base visible");
};

export default async ({ $, click, tick, expect }) => {
  expect(".base").attr("class", "base");
  await click(".base");
  expect(".base").attr("class", "base active");
};

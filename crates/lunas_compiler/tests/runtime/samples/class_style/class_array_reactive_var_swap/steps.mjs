export default async ({ $, click, expect }) => {
  expect(".base").attr("class", "base one");
  await click(".base");
  expect(".base").attr("class", "base two");
};

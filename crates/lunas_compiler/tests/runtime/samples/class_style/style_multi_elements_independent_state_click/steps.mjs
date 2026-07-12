export default async ({ $, click, expect }) => {
  expect(".one").attr("style", "color: red;");
  expect(".two").attr("style", "color: blue;");
  await click(".one");
  expect(".one").attr("style", "color: green;");
  expect(".two").attr("style", "color: blue;");
  await click(".two");
  expect(".two").attr("style", "color: yellow;");
};

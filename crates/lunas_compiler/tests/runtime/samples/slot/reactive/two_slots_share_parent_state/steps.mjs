export default async ({ $, click, expect }) => {
  expect(".d").text("d1");
  expect(".h").text("h1");
  await click("button");
  expect(".d").text("d2");
  expect(".h").text("h2");
};

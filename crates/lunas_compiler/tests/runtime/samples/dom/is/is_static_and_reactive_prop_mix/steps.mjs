export default async ({ click, expect }) => {
  expect(".kind").text("info");
  expect(".title").text("A");
  await click("button");
  expect(".kind").text("info");
  expect(".title").text("B");
};

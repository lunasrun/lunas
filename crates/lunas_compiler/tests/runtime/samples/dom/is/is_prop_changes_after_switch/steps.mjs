export default async ({ click, expect }) => {
  expect(".panel-label").text("hi");
  await click(".swap");
  expect(".notice-label").text("hi");
  await click(".rename");
  expect(".notice-label").text("yo");
};

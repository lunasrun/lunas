export default async ({ click, expect }) => {
  expect(".p1").text("0");
  expect(".p2").text("0");
  await click("button");
  expect(".p1").text("1");
  expect(".p2").text("1");
};

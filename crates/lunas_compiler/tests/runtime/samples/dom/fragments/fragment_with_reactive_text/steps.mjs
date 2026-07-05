export default async ({ click, expect }) => {
  expect(".left").text("0");
  expect(".right").text("100");
  await click("button");
  expect(".left").text("1");
  expect(".right").text("99");
};

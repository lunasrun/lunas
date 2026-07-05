export default async ({ $, click, expect }) => {
  expect(".chip").text("1");
  await click("button");
  expect(".chip").text("2");
};

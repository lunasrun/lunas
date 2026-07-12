export default async ({ $, click, expect }) => {
  expect("p").text("0");
  await click("button");
  expect("p").text("5");
  await click("button");
  expect("p").text("10");
};

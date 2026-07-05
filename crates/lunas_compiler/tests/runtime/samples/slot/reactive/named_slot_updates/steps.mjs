export default async ({ $, click, expect }) => {
  expect("b").text("0");
  await click("button");
  expect("b").text("1");
};

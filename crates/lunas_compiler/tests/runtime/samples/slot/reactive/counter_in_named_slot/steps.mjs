export default async ({ $, click, expect }) => {
  expect("button").text("v=5");
  await click("button");
  expect("button").text("v=6");
};

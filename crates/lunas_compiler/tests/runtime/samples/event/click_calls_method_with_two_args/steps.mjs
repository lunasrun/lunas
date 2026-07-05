export default async ({ $, click, expect }) => {
  expect("button").text("sum=0");
  await click("button");
  expect("button").text("sum=5");
};

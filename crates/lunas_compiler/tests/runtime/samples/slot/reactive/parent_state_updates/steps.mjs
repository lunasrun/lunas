export default async ({ $, click, expect }) => {
  expect("span").text("n=1");
  await click("button");
  expect("span").text("n=2");
  await click("button");
  expect("span").text("n=3");
};

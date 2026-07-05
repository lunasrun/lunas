export default async ({ $, click, expect }) => {
  expect("button").text("n=2");
  await click("button");
  expect("button").text("n=6");
};

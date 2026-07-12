export default async ({ $, click, expect }) => {
  expect("button").text("n=0");
  await click("button");
  expect("button").text("n=1");
};

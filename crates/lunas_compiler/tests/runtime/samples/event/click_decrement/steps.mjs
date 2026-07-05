export default async ({ $, click, expect }) => {
  expect("button").text("n=5");
  await click("button");
  expect("button").text("n=4");
};

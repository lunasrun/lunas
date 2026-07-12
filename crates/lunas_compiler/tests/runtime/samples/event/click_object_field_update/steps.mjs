export default async ({ $, click, expect }) => {
  expect("button").text("count=0");
  await click("button");
  expect("button").text("count=1");
};

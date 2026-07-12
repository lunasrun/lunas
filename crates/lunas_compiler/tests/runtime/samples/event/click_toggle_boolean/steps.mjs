export default async ({ $, click, expect }) => {
  expect("button").text("false");
  await click("button");
  expect("button").text("true");
  await click("button");
  expect("button").text("false");
};

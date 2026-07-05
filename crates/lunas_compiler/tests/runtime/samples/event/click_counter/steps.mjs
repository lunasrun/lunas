export default async ({ $, click, tick, expect }) => {
  expect("button").text("count: 2");
  await click("button");
  expect("button").text("count: 3");
  await click("button");
  expect("button").text("count: 4");
};

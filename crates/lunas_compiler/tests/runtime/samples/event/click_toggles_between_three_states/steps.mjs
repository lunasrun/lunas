export default async ({ click, expect }) => {
  expect("button").text("a");
  await click("button");
  expect("button").text("b");
  await click("button");
  expect("button").text("c");
  await click("button");
  expect("button").text("a");
};

export default async ({ click, expect }) => {
  expect("button").text("none");
  await click("button");
  expect("button").text("clicked 0");
  await click("button");
  expect("button").text("clicked 1");
};

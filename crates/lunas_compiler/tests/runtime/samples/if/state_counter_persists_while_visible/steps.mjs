export default async ({ click, expect }) => {
  expect("p").text("zero");
  await click("button");
  expect("p").text("count is 1");
  await click("button");
  expect("p").text("count is 2");
};

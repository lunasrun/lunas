export default async ({ click, expect }) => {
  expect("button").text("count: 5");
  await click("button");
  expect("button").text("count: 6");
};

export default async ({ click, expect }) => {
  expect("button").text("props=5");
  await click("button");
  expect("button").text("props=15");
};

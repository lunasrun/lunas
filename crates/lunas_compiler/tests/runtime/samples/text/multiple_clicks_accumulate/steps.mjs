export default async ({ click, expect }) => {
  expect("button").text("0");
  await click("button");
  await click("button");
  await click("button");
  await click("button");
  await click("button");
  expect("button").text("5");
};

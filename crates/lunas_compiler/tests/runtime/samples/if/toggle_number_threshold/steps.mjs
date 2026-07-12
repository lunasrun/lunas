export default async ({ click, expect }) => {
  expect("p").text("Below threshold (0)");
  await click("button");
  expect("p").text("Below threshold (1)");
  await click("button");
  expect("p").text("Below threshold (2)");
  await click("button");
  expect("p").text("Threshold reached");
};

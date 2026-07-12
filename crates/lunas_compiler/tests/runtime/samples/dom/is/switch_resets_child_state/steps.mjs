export default async ({ click, expect }) => {
  await click(".inc");
  await click(".inc");
  expect("span").text("2");
  await click(".swap");
  expect(".notice").count(1);
  await click(".back");
  // Fresh Counter instance -> state reset to 0.
  expect("span").text("0");
};

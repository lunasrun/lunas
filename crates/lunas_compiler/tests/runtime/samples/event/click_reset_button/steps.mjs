export default async ({ $, click, expect }) => {
  await click(".inc");
  await click(".inc");
  expect("span").text("2");
  await click(".reset");
  expect("span").text("0");
};

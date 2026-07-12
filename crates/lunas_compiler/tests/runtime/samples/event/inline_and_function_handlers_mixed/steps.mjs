export default async ({ click, expect }) => {
  expect("p").text("0");
  await click(".inc");
  await click(".inc");
  expect("p").text("2");
  await click(".reset");
  expect("p").text("0");
};

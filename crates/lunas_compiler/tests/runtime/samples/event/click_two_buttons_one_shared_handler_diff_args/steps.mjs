export default async ({ click, expect }) => {
  await click(".plus");
  await click(".plus");
  expect("span").text("2");
  await click(".minus");
  expect("span").text("1");
};

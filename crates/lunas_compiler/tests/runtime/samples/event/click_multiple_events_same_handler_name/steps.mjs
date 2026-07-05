export default async ({ click, expect }) => {
  await click(".a");
  await click(".b");
  await click(".a");
  expect("span").text("3");
};

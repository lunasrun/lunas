export default async ({ $, click, expect }) => {
  expect("span").text("none");
  await click(".b");
  expect("span").text("b");
  await click(".c");
  expect("span").text("c");
  await click(".a");
  expect("span").text("a");
};

export default async ({ $, click, expect }) => {
  expect("p").count(0);
  await click(".show");
  expect("p").text("shown");
  await click(".hide");
  expect("p").count(0);
};

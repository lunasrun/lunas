export default async ({ click, expect }) => {
  expect("b").count(0);
  await click("button");
  expect("b").count(1);
  expect("b").text("now shown");
  await click("button");
  expect("b").count(0);
};

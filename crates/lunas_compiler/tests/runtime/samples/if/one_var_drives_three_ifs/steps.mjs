export default async ({ click, expect }) => {
  expect("span").count(0);
  expect("em").count(0);
  expect("i").count(1);
  await click("button");
  expect("span").count(1);
  expect("em").count(1);
  expect("i").count(0);
};

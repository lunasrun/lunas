export default async ({ click, expect }) => {
  expect("span").count(0);
  await click("button");
  await click("button");
  expect("span").count(0);
  await click("button");
  expect("span").count(1);
};

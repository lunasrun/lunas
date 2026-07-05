export default async ({ click, expect }) => {
  expect("p").count(1);
  await click("button");
  expect("p").count(0);
};

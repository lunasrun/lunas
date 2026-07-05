export default async ({ click, expect }) => {
  expect(".panel").count(1);
  await click("button");
  expect(".notice").count(1);
  await click("button");
  expect(".panel").count(1);
  expect(".notice").count(0);
};

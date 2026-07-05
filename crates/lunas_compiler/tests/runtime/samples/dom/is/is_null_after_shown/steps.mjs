export default async ({ click, expect }) => {
  expect(".panel").count(1);
  await click(".hide");
  expect(".panel").count(0);
  await click(".show");
  expect(".panel").count(1);
  await click(".hide");
  expect(".panel").count(0);
};

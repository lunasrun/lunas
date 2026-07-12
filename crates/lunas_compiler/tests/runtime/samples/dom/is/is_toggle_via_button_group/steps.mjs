export default async ({ $$, click, expect }) => {
  const [red, green, blue] = $$("button");
  expect(".red").count(1);
  await click(green);
  expect(".red").count(0);
  expect(".green").count(1);
  await click(blue);
  expect(".green").count(0);
  expect(".blue").count(1);
  await click(red);
  expect(".blue").count(0);
  expect(".red").count(1);
};

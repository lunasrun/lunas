export default async ({ $$, click, expect }) => {
  const [bBtn, cBtn, aBtn] = $$("button");
  expect(".a").count(1);
  expect(".b").count(0);
  expect(".c").count(0);
  await click(bBtn);
  expect(".a").count(0);
  expect(".b").count(1);
  expect(".c").count(0);
  await click(cBtn);
  expect(".a").count(0);
  expect(".b").count(0);
  expect(".c").count(1);
  await click(aBtn);
  expect(".a").count(1);
  expect(".b").count(0);
  expect(".c").count(0);
};

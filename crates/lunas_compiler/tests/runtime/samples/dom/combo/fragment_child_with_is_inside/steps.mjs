export default async ({ click, expect }) => {
  expect("h2").text("fragment heading");
  expect(".foo").count(1);
  await click("button");
  expect(".foo").count(0);
  expect(".bar").count(1);
};

export default async ({ click, expect }) => {
  expect(".target").hasClass("target");
  await click("button");
  expect(".target").hasClass("active");
};

export default async ({ $, click, expect }) => {
  expect(".host").html("<p>zero</p>");
  await click("button");
  expect(".host").html("<p>one</p>");
  await click("button");
  expect(".host").html("");
};

export default async ({ $, click, expect }) => {
  expect(".host").html("");
  await click("button");
  expect(".host").html("<span>visible</span>");
  await click("button");
  expect(".host").html("");
};

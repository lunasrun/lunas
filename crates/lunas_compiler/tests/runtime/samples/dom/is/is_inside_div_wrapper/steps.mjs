export default async ({ click, expect }) => {
  expect(".inner").html("<div><p class=\"panel\">panel</p></div>");
  await click("button");
  expect(".inner").html("<div><p class=\"notice\">notice</p></div>");
};

export default async ({ click, expect }) => {
  expect("span").attr("data-state", null);
  await click("button");
  expect("span").attr("data-state", "marked");
};

export default async ({ expect, click }) => {
  expect("span").attr("tabindex", "0");
  await click("span");
  expect("span").attr("tabindex", "1");
};

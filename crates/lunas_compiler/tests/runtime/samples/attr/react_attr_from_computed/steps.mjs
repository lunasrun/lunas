export default async ({ expect, click }) => {
  expect("span").attr("title", "0 clicks");
  await click("span");
  expect("span").attr("title", "1 clicks");
};

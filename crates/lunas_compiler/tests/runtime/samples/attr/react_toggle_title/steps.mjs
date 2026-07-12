export default async ({ expect, click }) => {
  expect("span").attr("title", "first");
  await click("span");
  expect("span").attr("title", "second");
};

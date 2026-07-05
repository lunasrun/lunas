export default async ({ expect, click }) => {
  expect("span").attr("class", "box light");
  await click("span");
  expect("span").attr("class", "box dark");
};

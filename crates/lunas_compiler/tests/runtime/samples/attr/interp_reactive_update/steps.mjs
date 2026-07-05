export default async ({ expect, click }) => {
  expect("span").attr("class", "badge idle");
  await click("button");
  expect("span").attr("class", "badge busy");
};

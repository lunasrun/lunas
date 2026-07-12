export default async ({ click, expect }) => {
  expect("article").html("");
  await click("button");
  expect("article").html("<b>reference text</b>");
};

export default async ({ click, expect }) => {
  expect("article").html("<b>bold</b>");
  await click("button");
  expect("article").html("");
};

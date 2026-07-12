export default async ({ click, expect }) => {
  await click("button");
  expect("article").html("<b>A1</b>");
  expect("section").html("<b>B0</b>");
};

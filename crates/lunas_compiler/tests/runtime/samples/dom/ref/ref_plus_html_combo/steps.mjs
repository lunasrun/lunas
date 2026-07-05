export default async ({ click, expect }) => {
  expect("article").html("<b>bold</b>");
  await click("button");
  expect("article").html("<b>bold</b><i>more</i>");
  expect("article").attr("data-grown", "yes");
};

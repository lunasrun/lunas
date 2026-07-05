export default async ({ click, expect }) => {
  expect("article").html("<b>bold</b>");
  await click("button");
  expect("article").attr("data-touched", "yes");
  expect("article").html("<b>bold</b>");
};

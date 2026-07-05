export default async ({ click, expect }) => {
  expect("h3").text("Title");
  expect("article").html("<b>bold body</b>");
  await click("button");
  expect("h3").text("Updated");
  expect("article").html("<i>italic body</i>");
};

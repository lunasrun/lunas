export default async ({ click, expect }) => {
  expect("article").html("<b>one</b>");
  expect("section").html("<i>two</i>");
  await click("button");
  expect("article").html("<i>two</i>");
  expect("section").html("<b>one</b>");
};

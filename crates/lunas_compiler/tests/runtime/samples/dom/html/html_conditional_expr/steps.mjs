export default async ({ click, expect }) => {
  expect("article").html("<b>on</b>");
  await click("button");
  expect("article").html("<i>off</i>");
};

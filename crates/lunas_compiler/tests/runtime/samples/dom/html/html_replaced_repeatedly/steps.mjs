export default async ({ click, expect }) => {
  expect("article").html("<b>0</b>");
  await click("button");
  expect("article").html("<b>1</b>");
  await click("button");
  expect("article").html("<b>2</b>");
  await click("button");
  expect("article").html("<b>3</b>");
};

export default async ({ $$, click, expect }) => {
  await click("button");
  expect("ul").text("<li>0</li><li>1</li>");
};

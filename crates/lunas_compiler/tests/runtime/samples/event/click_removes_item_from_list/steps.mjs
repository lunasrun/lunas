export default async ({ click, expect }) => {
  expect("ul").text("<li>1</li><li>2</li><li>3</li>");
  await click("button");
  expect("ul").text("<li>2</li><li>3</li>");
};

export default async ({ click, expect }) => {
  expect("article").html("<p>first note</p>");
  await click("button");
  expect("article").html("<p>edited note</p>");
};

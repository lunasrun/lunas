export default async ({ click, expect }) => {
  expect("span").text("one");
  await click("button");
  expect("span").text("two");
};

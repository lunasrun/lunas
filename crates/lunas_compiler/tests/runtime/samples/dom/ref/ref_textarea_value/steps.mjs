export default async ({ click, expect }) => {
  await click("button");
  expect("textarea").value("hello there");
};

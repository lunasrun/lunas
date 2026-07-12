export default async ({ click, expect }) => {
  expect("a").text("false");
  await click("a");
  expect("a").text("true");
};

export default async ({ click, expect }) => {
  expect("label").text("false");
  await click("label");
  expect("label").text("true");
};

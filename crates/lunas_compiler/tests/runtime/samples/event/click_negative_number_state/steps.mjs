export default async ({ click, expect }) => {
  await click("button");
  await click("button");
  expect("button").text("n=-2");
};

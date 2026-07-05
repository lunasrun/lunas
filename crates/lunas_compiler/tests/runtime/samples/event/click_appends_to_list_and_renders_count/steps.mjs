export default async ({ click, expect }) => {
  expect("span").text("0");
  await click("button");
  await click("button");
  expect("span").text("2");
};

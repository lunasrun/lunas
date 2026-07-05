export default async ({ click, expect }) => {
  expect("span").text("1");
  await click("button");
  expect("span").text("2");
};

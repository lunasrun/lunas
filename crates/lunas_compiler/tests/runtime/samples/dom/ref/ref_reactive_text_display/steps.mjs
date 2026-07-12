export default async ({ click, expect }) => {
  expect("span").text("");
  await click("button");
  expect("span").text("start");
};

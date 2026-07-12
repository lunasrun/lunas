export default async ({ $, click, expect }) => {
  expect("button").text("s=a");
  await click("button");
  expect("button").text("s=ab");
};

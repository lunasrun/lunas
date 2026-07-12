export default async ({ click, expect }) => {
  expect("button").text("0-10");
  await click("button");
  expect("button").text("1-11");
  await click("button");
  expect("button").text("2-12");
};

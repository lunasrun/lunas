export default async ({ click, expect }) => {
  expect("button").text("a");
  await click("button");
  expect("button").text("aa");
  await click("button");
  expect("button").text("aaa");
};

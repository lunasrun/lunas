export default async ({ click, expect }) => {
  expect("button").text("v: 1");
  await click("button");
  expect("button").text("v: 11");
};

export default async ({ click, expect }) => {
  expect("button").text("c=0");
  await click("button");
  expect("button").text("c=1");
  await click("button");
  expect("button").text("c=2");
};

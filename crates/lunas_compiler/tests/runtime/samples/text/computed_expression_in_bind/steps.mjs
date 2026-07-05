export default async ({ click, expect }) => {
  expect("button").text("double: 2");
  await click("button");
  expect("button").text("double: 4");
};

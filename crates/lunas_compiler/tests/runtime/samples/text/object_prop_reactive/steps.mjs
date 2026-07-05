export default async ({ click, expect }) => {
  expect("p").text("A");
  await click("button");
  expect("p").text("B");
};

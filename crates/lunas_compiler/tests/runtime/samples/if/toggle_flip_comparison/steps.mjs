export default async ({ click, expect }) => {
  expect("p").text("Small");
  await click("button");
  expect("p").text("Small");
  await click("button");
  expect("p").text("Big");
};

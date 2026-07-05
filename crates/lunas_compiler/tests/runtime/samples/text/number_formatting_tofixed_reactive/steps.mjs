export default async ({ click, expect }) => {
  expect("p").text("1.00");
  await click("button");
  expect("p").text("1.10");
};

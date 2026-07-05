export default async ({ click, expect }) => {
  expect("span").text("hi");
  await click("button");
  expect("span").text("yo");
};

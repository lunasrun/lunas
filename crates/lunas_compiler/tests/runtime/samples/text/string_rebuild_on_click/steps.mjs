export default async ({ click, expect }) => {
  expect("p").text("Hello Ann");
  await click("button");
  expect("p").text("Hello Bob");
};

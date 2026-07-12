export default async ({ click, expect }) => {
  expect("span").text("n=0");
  await click("button");
  expect("span").text("n=1");
};

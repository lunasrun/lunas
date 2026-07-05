export default async ({ click, expect }) => {
  await click("button");
  expect("button").text("n=1");
};

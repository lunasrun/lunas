export default async ({ $, click, expect }) => {
  expect("button").text("n=0 double=0");
  await click("button");
  expect("button").text("n=1 double=2");
};

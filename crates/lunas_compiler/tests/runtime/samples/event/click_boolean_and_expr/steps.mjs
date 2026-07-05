export default async ({ click, expect }) => {
  expect("button").text("show=false");
  await click("button");
  await click("button");
  await click("button");
  expect("button").text("show=true");
};

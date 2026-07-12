export default async ({ click, expect }) => {
  expect("span").text("ok");
  await click("button");
  await click("button");
  await click("button");
  expect("span").text("full");
};

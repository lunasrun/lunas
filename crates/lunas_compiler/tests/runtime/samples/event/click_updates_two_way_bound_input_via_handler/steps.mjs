export default async ({ $, click, expect }) => {
  expect("input").value("alice");
  await click("button");
  expect("input").value("bob");
};

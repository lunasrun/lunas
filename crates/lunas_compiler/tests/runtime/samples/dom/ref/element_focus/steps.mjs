export default async ({ $, click, expect }) => {
  await click("button");
  expect("input").value("typed");
};

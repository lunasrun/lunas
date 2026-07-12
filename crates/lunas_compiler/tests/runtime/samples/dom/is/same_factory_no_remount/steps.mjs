export default async ({ $, $$, click, expect }) => {
  await click(".inc");
  await click(".inc");
  expect("span").text("2");
  const [, reassignBtn] = $$("button");
  await click(reassignBtn);
  // Setting :is to the SAME factory must not remount -> state preserved.
  expect("span").text("2");
};

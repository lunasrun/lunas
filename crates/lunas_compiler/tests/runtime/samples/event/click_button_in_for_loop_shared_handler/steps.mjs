export default async ({ $$, click, expect }) => {
  const buttons = $$(".btn");
  await click(buttons[1]);
  expect("span").text("2");
  await click(buttons[2]);
  expect("span").text("5");
};

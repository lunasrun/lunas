export default async ({ $$, click, expect }) => {
  expect(".text-editor").count(1);
  const [textBtn, numberBtn, toggleBtn] = $$("button");
  await click(numberBtn);
  expect(".text-editor").count(0);
  expect(".number-editor").count(1);
  await click(toggleBtn);
  expect(".number-editor").count(0);
  expect(".toggle-editor").count(1);
  await click(textBtn);
  expect(".toggle-editor").count(0);
  expect(".text-editor").count(1);
};

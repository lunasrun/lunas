export default async ({ $$, click, expect }) => {
  expect("button.b").count(2);
  await click($$("button.b")[1]);
};

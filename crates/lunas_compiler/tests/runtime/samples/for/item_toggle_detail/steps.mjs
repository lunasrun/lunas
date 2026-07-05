export default async ({ $$, click, expect }) => {
  expect("span.d").count(0);
  await click($$("button.t")[0]);
  expect("span.d").count(1);
  await click($$("button.t")[1]);
  expect("span.d").count(2);
};

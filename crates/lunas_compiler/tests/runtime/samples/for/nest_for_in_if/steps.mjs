export default async ({ $$, click, expect }) => {
  expect("li").count(0);
  await click(".go");
  expect("li").count(3);
  await click(".go");
  expect("li").count(0);
};

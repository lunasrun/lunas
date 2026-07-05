export default async ({ $$, click, expect }) => {
  expect("li").count(2);
  expect("p.empty").count(0);
  await click(".go");
  expect("li").count(0);
  expect("p.empty").count(1);
};

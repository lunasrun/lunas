export default async ({ $$, click, expect }) => {
  expect("span").count(0);
  await click($$("button")[0]);
  expect("span").count(1);
  expect("span").text("count: 0");
  const bumpBtn = $$("button")[1];
  await click(bumpBtn);
  expect("span").text("count: 1");
  await click(bumpBtn);
  expect("span").text("count: 2");
};

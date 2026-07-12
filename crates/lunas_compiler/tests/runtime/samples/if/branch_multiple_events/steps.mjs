export default async ({ $$, click, expect }) => {
  expect("span").count(0);
  await click($$("button")[0]);
  expect("span").text("5");
  const [, inc, dec] = $$("button");
  await click(inc);
  expect("span").text("6");
  await click(dec);
  await click(dec);
  expect("span").text("4");
};

export default async ({ $$, click, expect }) => {
  expect("button").count(1);
  await click($$("button")[0]);
  expect("button").count(2);
  const plus = $$("button")[1];
  expect(plus).text("plus (0)");
  await click(plus);
  expect(plus).text("plus (1)");
  await click(plus);
  expect(plus).text("plus (2)");
};

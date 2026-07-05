export default async ({ $$, click, expect }) => {
  const [b1, b2] = $$("button");
  expect(b1).text("1");
  expect(b2).text("100");
  await click(b1);
  expect(b1).text("2");
  expect(b2).text("100");
  await click(b2);
  expect(b1).text("2");
  expect(b2).text("101");
};

export default async ({ $$, click, expect }) => {
  const [a, b] = $$("button");
  await click(a);
  await click(a);
  expect(a).text("2");
  expect(b).text("0");
};

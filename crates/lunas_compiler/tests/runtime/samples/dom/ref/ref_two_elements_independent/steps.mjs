export default async ({ $$, click, expect }) => {
  const [inputA, inputB] = $$("input");
  const [btnA, btnB] = $$("button");
  await click(btnA);
  expect(inputA).value("A");
  expect(inputB).value("");
  await click(btnB);
  expect(inputB).value("B");
};

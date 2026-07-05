export default async ({ $$, setValue, expect }) => {
  const inputs = $$("input");
  expect(inputs[0]).value("a");
  expect(inputs[1]).value("b");
  await setValue(inputs[1], "changed");
  expect(inputs[1]).value("changed");
  expect(inputs[0]).value("a");
};

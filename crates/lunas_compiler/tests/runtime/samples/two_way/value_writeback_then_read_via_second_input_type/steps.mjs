export default async ({ setValue, $$, expect }) => {
  const inputs = $$("input");
  expect(inputs[1]).value("orig");
  await setValue(inputs[0], "changed");
  expect(inputs[1]).value("changed");
};

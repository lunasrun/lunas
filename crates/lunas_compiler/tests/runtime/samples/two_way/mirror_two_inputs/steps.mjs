export default async ({ $$, setValue, tick, expect }) => {
  const inputs = $$("input");
  await setValue(inputs[0], "typed");
  expect(inputs[1]).value("typed");
};

export default async ({ $$, setValue, equal, expect }) => {
  expect("input.a").count(2);
  expect("input.b").count(2);
  await setValue($$("input.a")[0], "hi");
  await setValue($$("input.b")[1], "yo");
  equal($$("input.a")[0].value, "hi");
  equal($$("input.b")[1].value, "yo");
};

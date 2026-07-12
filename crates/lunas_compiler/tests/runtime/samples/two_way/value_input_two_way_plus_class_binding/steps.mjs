export default async ({ setValue, expect }) => {
  expect("input").attr("class", "fld");
  await setValue("input", "hi");
  expect("input").attr("class", "fld filled");
};

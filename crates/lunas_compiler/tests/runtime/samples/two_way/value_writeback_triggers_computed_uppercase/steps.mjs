export default async ({ setValue, expect }) => {
  expect("span").text("ABC");
  await setValue("input", "xyz");
  expect("span").text("XYZ");
};

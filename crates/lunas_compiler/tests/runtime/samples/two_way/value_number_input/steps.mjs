export default async ({ setValue, expect }) => {
  expect("span").text("0");
  await setValue("input", "42");
  expect("span").text("42");
};

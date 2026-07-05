export default async ({ setValue, expect }) => {
  await setValue("input", "0");
  expect("span").text("[0]");
};

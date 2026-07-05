export default async ({ setValue, expect }) => {
  await setValue("input", "2");
  await setValue("input", "3");
  expect("span").text("3");
};

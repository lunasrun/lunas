export default async ({ setValue, expect }) => {
  await setValue("input", "555-1234");
  expect("span").text("555-1234");
};

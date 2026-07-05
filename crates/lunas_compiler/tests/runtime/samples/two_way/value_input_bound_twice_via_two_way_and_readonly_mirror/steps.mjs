export default async ({ setValue, expect }) => {
  expect("p").count(0);
  await setValue("input", "hi");
  expect("p").count(0);
  await setValue("input", "hello");
  expect("p").text("long enough");
};

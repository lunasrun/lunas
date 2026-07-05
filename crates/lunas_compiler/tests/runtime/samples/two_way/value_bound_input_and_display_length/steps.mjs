export default async ({ setValue, expect }) => {
  expect("span").text("len=2");
  await setValue("input", "hello");
  expect("span").text("len=5");
};

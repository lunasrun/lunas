export default async ({ setValue, expect }) => {
  expect("span").text("double=2");
  await setValue("input", "5");
  expect("span").text("double=10");
};

export default async ({ setValue, expect }) => {
  expect("span").text("x");
  await setValue("input", "typed-value");
  expect("span").text("typed-value");
};

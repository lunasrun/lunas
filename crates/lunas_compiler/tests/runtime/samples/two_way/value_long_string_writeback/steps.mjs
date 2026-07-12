export default async ({ setValue, expect }) => {
  const long = "x".repeat(50);
  await setValue("input", long);
  expect("span").text("50");
};

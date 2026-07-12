export default async ({ setValue, expect }) => {
  await setValue("input", "こんにちは");
  expect("span").text("こんにちは");
};

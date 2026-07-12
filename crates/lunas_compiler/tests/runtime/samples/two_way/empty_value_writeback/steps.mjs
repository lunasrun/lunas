export default async ({ setValue, expect }) => {
  expect("span").text("[nonempty]");
  await setValue("input", "");
  expect("span").text("[]");
};

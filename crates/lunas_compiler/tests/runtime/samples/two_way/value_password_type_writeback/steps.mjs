export default async ({ setValue, expect }) => {
  await setValue("input", "secret1");
  expect("span").text("len=7");
};

export default async ({ setValue, expect }) => {
  await setValue("input", "a@b.com");
  expect("span").text("a@b.com");
};

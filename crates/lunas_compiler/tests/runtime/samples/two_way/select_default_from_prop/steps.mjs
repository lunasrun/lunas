export default async ({ setValue, expect }) => {
  expect("span").text("b");
  await setValue("select", "a");
  expect("span").text("a");
};

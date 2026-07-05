export default async ({ setValue, expect }) => {
  expect("span").text("x");
  await setValue("select", "y");
  expect("span").text("y");
};

export default async ({ setValue, expect }) => {
  expect("span").text("1!");
  await setValue("input", "2");
  expect("span").text("2!");
};

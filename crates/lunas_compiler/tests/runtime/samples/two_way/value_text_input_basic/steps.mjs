export default async ({ setValue, expect }) => {
  expect("span").text("hi");
  await setValue("input", "bye");
  expect("span").text("bye");
};

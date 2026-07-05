export default async ({ setValue, expect }) => {
  expect("p").text("echo: ");
  await setValue("input", "hello world");
  expect("p").text("echo: hello world");
};

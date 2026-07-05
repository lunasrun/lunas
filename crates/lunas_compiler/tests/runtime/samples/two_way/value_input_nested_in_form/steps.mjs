export default async ({ setValue, expect }) => {
  expect("p").text("Hello, Ada");
  await setValue("input", "Grace");
  expect("p").text("Hello, Grace");
};

export default async ({ setValue, expect }) => {
  await setValue("input", "https://example.com");
  expect("span").text("https://example.com");
};

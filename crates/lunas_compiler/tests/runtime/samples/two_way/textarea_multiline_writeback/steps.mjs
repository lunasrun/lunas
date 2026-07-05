export default async ({ setValue, expect }) => {
  await setValue("textarea", "line1\nline2");
  expect("span").text("line1\nline2");
};

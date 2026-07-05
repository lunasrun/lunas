export default async ({ setValue, expect }) => {
  await setValue("input", "lunas");
  expect("span").text("q=lunas");
};

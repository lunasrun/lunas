export default async ({ setValue, expect }) => {
  await setValue("input", "a");
  expect("span").text("e=1 t=a");
  await setValue("input", "ab");
  expect("span").text("e=2 t=ab");
};

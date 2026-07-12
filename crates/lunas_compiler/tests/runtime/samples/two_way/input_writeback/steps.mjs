export default async ({ $, setValue, tick, expect }) => {
  expect("span").text("a");
  await setValue("input", "zed");
  expect("span").text("zed");
};

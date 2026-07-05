export default async ({ setValue, expect }) => {
  expect("span").text("seed");
  await setValue("input", "changed");
  expect("span").text("changed");
};

export default async ({ setValue, expect }) => {
  expect("span").text("initial");
  await setValue("input", "updated");
  expect("span").text("updated");
};

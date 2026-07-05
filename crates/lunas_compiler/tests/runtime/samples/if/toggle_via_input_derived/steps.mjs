export default async ({ setValue, expect }) => {
  expect("p").text("Type to search");
  await setValue("input", "cats");
  expect("p").text("Searching: cats");
  await setValue("input", "");
  expect("p").text("Type to search");
};

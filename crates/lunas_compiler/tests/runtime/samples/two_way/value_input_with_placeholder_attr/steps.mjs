export default async ({ setValue, expect }) => {
  expect("input").attr("placeholder", "type here");
  await setValue("input", "filled");
  expect("span").text("filled");
};

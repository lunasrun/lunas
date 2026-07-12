export default async ({ setValue, click, expect }) => {
  await setValue("input", "draft");
  expect("p").text("saved: ");
  await click("button");
  expect("p").text("saved: draft");
};

export default async ({ expect, click }) => {
  expect("input").prop("checked", false);
  await click("input");
  expect("input").prop("checked", true);
};

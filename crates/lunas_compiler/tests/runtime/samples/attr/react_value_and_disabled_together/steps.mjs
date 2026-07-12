export default async ({ expect, click }) => {
  expect("input").value("x");
  expect("input").prop("disabled", true);
  await click("input");
  expect("input").prop("disabled", false);
};

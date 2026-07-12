export default async ({ expect, click }) => {
  expect("button").prop("disabled", false);
  await click("button");
  expect("button").prop("disabled", true);
  await click("button");
  expect("button").prop("disabled", false);
};

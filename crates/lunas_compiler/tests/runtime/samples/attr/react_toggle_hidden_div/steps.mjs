export default async ({ expect, click }) => {
  expect("p").prop("hidden", false);
  await click("button");
  expect("p").prop("hidden", true);
};

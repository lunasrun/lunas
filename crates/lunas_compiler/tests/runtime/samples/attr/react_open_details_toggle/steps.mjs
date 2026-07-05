export default async ({ expect, click }) => {
  expect("details").attr("open", "false");
  await click("summary");
  expect("details").attr("open", "true");
};

export default async ({ expect, click }) => {
  expect("input").value("v1");
  await click("button");
  expect("input").value("v2");
};

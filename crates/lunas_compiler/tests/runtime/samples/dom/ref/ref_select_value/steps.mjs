export default async ({ click, expect }) => {
  await click("button");
  expect("select").value("b");
};

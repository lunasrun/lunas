export default async ({ expect, click }) => {
  expect("a").attr("href", "/a");
  await click("a");
  expect("a").attr("href", "/b");
};

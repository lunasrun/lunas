export default async ({ click, expect }) => {
  await click(".inner");
  expect("span").text("o=0 i=1");
};

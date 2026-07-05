export default async ({ click, expect }) => {
  await click("button");
  expect("p").hasClass("hot");
};

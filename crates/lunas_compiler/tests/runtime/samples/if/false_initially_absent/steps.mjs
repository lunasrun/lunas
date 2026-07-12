export default async ({ click, expect }) => {
  expect("p").count(0);
  await click("button");
  expect("p").count(1);
  expect("p").text("Now visible");
};

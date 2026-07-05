export default async ({ expect, click }) => {
  expect("header").text("Header");
  expect("footer").text("Footer");
  expect("p").count(0);
  await click("button");
  expect("header").text("Header");
  expect("footer").text("Footer");
  expect("p").count(1);
  expect("p").text("Middle content");
};

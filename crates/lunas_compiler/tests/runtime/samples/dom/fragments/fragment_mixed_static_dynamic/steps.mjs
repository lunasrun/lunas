export default async ({ click, expect }) => {
  expect("header").text("static header");
  expect("p").text("start");
  await click("button");
  expect("p").text("changed");
  expect("footer").text("static footer");
};

export default async ({ click, expect }) => {
  expect("h3").text("Title A");
  expect("p").text("Sub A");
  await click("button");
  expect("h3").text("Title B");
  expect("p").text("Sub B");
};

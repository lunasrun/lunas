export default async ({ click, expect }) => {
  expect("p").text("OFF");
  await click("button");
  expect("p").text("ON");
};

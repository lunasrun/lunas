export default async ({ click, expect }) => {
  await click("button");
  expect("p").attr("style", "color: red;");
  expect("p").attr("data-painted", "yes");
};

export default async ({ click, expect }) => {
  await click("button");
  expect("p").attr("data-marked", "yes");
};

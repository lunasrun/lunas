export default async ({ click, expect }) => {
  await click("button");
  expect("span").text("has ref: yes");
};

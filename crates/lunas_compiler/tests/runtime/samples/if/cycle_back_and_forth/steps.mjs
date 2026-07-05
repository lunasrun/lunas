export default async ({ $$, click, expect }) => {
  const [left, right] = $$("button");
  expect("p").text("Middle");
  await click(right);
  expect("p").text("Right");
  await click(left);
  expect("p").text("Middle");
  await click(left);
  expect("p").text("Left");
  await click(left);
  expect("p").text("Left");
};

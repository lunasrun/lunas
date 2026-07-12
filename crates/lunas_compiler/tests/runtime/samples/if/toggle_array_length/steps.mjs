export default async ({ $$, click, expect }) => {
  const [add, clear] = $$("button");
  expect("p").text("No items");
  await click(add);
  expect("p").text("1 items");
  await click(add);
  expect("p").text("2 items");
  await click(clear);
  expect("p").text("No items");
};

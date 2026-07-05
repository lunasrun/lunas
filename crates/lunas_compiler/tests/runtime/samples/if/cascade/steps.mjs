export default async ({ $$, click, tick, expect }) => {
  const [one, two, x] = $$("button");
  expect("p").text("Page one");
  await click(two);
  expect("p").text("Page two");
  await click(x);
  expect("p").text("No page");
  await click(one);
  expect("p").text("Page one");
};

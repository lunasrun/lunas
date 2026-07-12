export default async ({ $$, click, expect }) => {
  const [a, b] = $$("button");
  expect("p").text("None");
  await click(a);
  expect("p").text("Panel A");
  await click(b);
  expect("p").text("Panel B");
  await click(b);
  expect("p").text("Panel B");
};

export default async ({ $$, click, equal, expect }) => {
  const labels = () => $$("button.d").map(n => n.innerHTMLString()).join(",");
  await click($$("button.d")[0]);
  equal(labels(), "b");
  await click($$("button.d")[0]); // remaining handler still valid
  expect("button.d").count(0);
};

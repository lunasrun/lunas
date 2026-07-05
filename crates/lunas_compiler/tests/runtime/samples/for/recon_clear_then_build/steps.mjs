export default async ({ $$, click, equal, expect }) => {
  const L = () => $$("li").map(n => n.innerHTMLString()).join(",");
  await click(".clr");
  expect("li").count(0);
  await click(".bld");
  equal(L(), "7,8,9");
};

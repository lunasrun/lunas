export default async ({ $$, click, equal, expect }) => {
  const L = () => $$("li").map(n => n.innerHTMLString()).join(",");
  await click(".go");
  equal(L(), "");
  expect("li").count(0);
};

export default async ({ $$, click, equal }) => {
  const L = () => $$("li").map(n => n.innerHTMLString()).join(",");
  await click(".drop");
  equal(L(), "b");
  await click(".add");
  equal(L(), "a2,b");
};

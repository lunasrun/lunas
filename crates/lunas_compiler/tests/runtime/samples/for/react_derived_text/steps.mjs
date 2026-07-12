export default async ({ $$, click, equal }) => {
  const L = () => $$("li").map(n => n.innerHTMLString()).join(",");
  equal(L(), "3,7");
  await click(".go");
  equal(L(), "12,34");
};

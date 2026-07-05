export default async ({ $$, $, click, equal }) => {
  const L = () => $$("li").map(n => n.innerHTMLString()).join(",");
  const first = $$("li")[0];
  equal(L(), "a,b,c");
  await click(".go");
  equal(L(), "a,b,c,d");
  equal($$("li")[0] === first, true); // unmoved head keeps identity
};

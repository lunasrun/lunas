export default async ({ $$, click, equal }) => {
  const L = () => $$("li").map(n => n.innerHTMLString()).join(",");
  const one = $$("li")[0];
  await click(".go");
  equal(L(), "1,2,3,4,5");
  equal($$("li")[0] === one, true);
};

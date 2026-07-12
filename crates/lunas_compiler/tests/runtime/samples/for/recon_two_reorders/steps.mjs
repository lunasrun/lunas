export default async ({ $$, click, equal }) => {
  const L = () => $$("li").map(n => n.innerHTMLString()).join(",");
  await click(".go");
  equal(L(), "c,a,b");
  await click(".go"); // applies again: was c,a,b -> [b,c,a]
  equal(L(), "b,c,a");
};

export default async ({ $$, click, equal }) => {
  const L = () => $$("li").map(n => n.innerHTMLString()).join(",");
  const [a] = $$("li");
  await click(".go");
  equal(L(), "z,a,b,c");
  equal($$("li")[1]===a, true);
};

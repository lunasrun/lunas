export default async ({ $$, click, equal }) => {
  const L = () => $$("li").map(n => n.innerHTMLString()).join(",");
  const nodes = {};
  for (const n of $$("li")) nodes[n.innerHTMLString()] = n;
  await click(".go");
  equal(L(), "a,b,c");
  const now = $$("li");
  equal(now[0]===nodes["a"] && now[1]===nodes["b"] && now[2]===nodes["c"], true);
};

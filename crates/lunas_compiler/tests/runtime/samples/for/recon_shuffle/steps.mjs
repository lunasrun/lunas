export default async ({ $$, click, equal }) => {
  const L = () => $$("li").map(n => n.innerHTMLString()).join(",");
  const map = {};
  for (const n of $$("li")) map[n.innerHTMLString()] = n;
  await click(".go");
  equal(L(), "d,a,e,b,c");
  const now = $$("li");
  equal(now[0]===map["d"] && now[1]===map["a"] && now[2]===map["e"] && now[3]===map["b"] && now[4]===map["c"], true);
};

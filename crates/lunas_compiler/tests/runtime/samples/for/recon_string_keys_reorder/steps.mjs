export default async ({ $$, click, equal }) => {
  const L = () => $$("li").map(n => n.innerHTMLString()).join(",");
  const map = {};
  for (const n of $$("li")) map[n.innerHTMLString()] = n;
  await click(".go");
  equal(L(), "c,b,a");
  const now = $$("li");
  equal(now[0]===map["c"] && now[2]===map["a"], true);
};

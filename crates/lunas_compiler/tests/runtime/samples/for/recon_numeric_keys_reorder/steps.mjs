export default async ({ $$, click, equal }) => {
  const L = () => $$("li").map(n => n.innerHTMLString()).join(",");
  const map = {};
  for (const n of $$("li")) map[n.innerHTMLString()] = n;
  await click(".go");
  equal(L(), "3,1,2");
  const now = $$("li");
  equal(now[0]===map["3"] && now[1]===map["1"] && now[2]===map["2"], true);
};

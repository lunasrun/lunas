export default async ({ $$, click, equal }) => {
  const L = () => $$("li").map(n => n.innerHTMLString()).join(",");
  const map = {};
  for (const n of $$("li")) map[n.innerHTMLString()] = n;
  await click(".go");
  equal(L(), "2,3,4,5,1");
  const now = $$("li");
  // 2,3,4,5 form the LIS and stay; only "1" moved to the end, all reused
  equal(now[4]===map["1"] && now[0]===map["2"] && now[3]===map["5"], true);
};

export default async ({ $$, click, equal }) => {
  const L = () => $$("li").map(n => n.innerHTMLString()).join(",");
  const o = $$("li").slice();
  await click(".go");
  equal(L(), "e,a,b,c,d");
  const now = $$("li");
  // a,b,c,d are the LIS and keep identity; only e moved
  equal(now[1]===o[0] && now[2]===o[1] && now[3]===o[2] && now[4]===o[3], true);
  equal(now[0]===o[4], true);
};

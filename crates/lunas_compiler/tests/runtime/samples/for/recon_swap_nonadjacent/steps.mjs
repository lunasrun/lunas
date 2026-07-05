export default async ({ $$, click, equal }) => {
  const L = () => $$("li").map(n => n.innerHTMLString()).join(",");
  const o = $$("li").slice();
  await click(".go");
  equal(L(), "e,b,c,d,a");
  const now = $$("li");
  equal(now[1]===o[1] && now[2]===o[2] && now[3]===o[3], true); // middle 3 unmoved
  equal(now[0]===o[4] && now[4]===o[0], true);
};

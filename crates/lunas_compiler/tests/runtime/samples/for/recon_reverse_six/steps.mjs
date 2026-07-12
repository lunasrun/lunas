export default async ({ $$, click, equal }) => {
  const L = () => $$("li").map(n => n.innerHTMLString()).join(",");
  const orig = $$("li").slice();
  await click(".go");
  equal(L(), "f,e,d,c,b,a");
  const now = $$("li");
  // every node reused (reverse preserves all identities)
  equal(now[0]===orig[5] && now[5]===orig[0], true);
};

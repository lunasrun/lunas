export default async ({ $$, click, equal }) => {
  const L = () => $$("li").map(n => n.innerHTMLString()).join(",");
  const [a,b,c] = $$("li");
  await click(".go");
  equal(L(), "a,c,b");
  const now = $$("li");
  equal(now[0]===a, true);      // unmoved keeps identity
  equal(now[1]===c && now[2]===b, true); // swapped nodes reused, not recreated
};

export default async ({ $$, click, equal }) => {
  const L = () => $$("li").map(n => n.innerHTMLString()).join(",");
  const [a,b,c] = $$("li");
  await click(".go");
  equal(L(), "c,b,a");
  const now = $$("li");
  equal(now[1]===b, true);           // middle unmoved keeps identity
  equal(now[0]===c && now[2]===a, true);
};

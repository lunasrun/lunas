export default async ({ $$, click, equal }) => {
  const L = () => $$("li").map(n => n.innerHTMLString()).join(",");
  const [a,b,c] = $$("li");
  await click(".go");
  equal(L(), "c,b,a");
  const now = $$("li");
  equal(now[0]===c && now[1]===b && now[2]===a, true); // all reused
};

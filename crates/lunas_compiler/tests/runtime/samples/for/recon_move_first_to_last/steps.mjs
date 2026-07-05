export default async ({ $$, click, equal }) => {
  const L = () => $$("li").map(n => n.innerHTMLString()).join(",");
  const [a,b,c] = $$("li");
  await click(".go");
  equal(L(), "b,c,a");
  const now = $$("li");
  equal(now[2]===a, true);      // moved node reused
  equal(now[0]===b && now[1]===c, true);
};

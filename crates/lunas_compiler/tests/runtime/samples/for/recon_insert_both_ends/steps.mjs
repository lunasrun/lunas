export default async ({ $$, click, equal }) => {
  const L = () => $$("li").map(n => n.innerHTMLString()).join(",");
  const [a,b,c] = $$("li");
  await click(".go");
  equal(L(), "z,a,b,c,d");
  const now = $$("li");
  equal(now[1]===a && now[2]===b && now[3]===c, true);
};

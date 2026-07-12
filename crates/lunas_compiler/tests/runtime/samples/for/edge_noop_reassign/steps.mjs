export default async ({ $$, click, equal }) => {
  const L = () => $$("li").map(n => n.innerHTMLString()).join(",");
  const [a,b,c] = $$("li");
  await click(".go");
  equal(L(), "a,b,c");
  const now = $$("li");
  equal(now[0]===a && now[1]===b && now[2]===c, true); // identical order -> nothing moves
};

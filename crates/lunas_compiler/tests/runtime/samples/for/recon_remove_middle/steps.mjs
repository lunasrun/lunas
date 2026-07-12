export default async ({ $$, click, equal }) => {
  const L = () => $$("li").map(n => n.innerHTMLString()).join(",");
  const [a,,c] = $$("li");
  await click(".go");
  equal(L(), "a,c");
  const now = $$("li");
  equal(now[0]===a && now[1]===c, true);
};

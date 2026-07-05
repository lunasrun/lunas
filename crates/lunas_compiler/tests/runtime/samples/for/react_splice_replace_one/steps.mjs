export default async ({ $$, click, equal }) => {
  const L = () => $$("li").map(n => n.innerHTMLString()).join(",");
  const [a,,c] = $$("li");
  await click(".go");
  equal(L(), "a,X,c");
  const now = $$("li");
  equal(now[0]===a && now[2]===c, true); // neighbors kept, middle replaced
};

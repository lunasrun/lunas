export default async ({ $$, click, equal }) => {
  await click(".go");
  const vs = $$("li").map(n => n.getAttribute("data-v")).join(",");
  equal(vs, "xx,yy");
};

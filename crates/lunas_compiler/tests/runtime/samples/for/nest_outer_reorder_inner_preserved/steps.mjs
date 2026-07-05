export default async ({ $$, click, equal }) => {
  const gids = () => $$("b.gid").map(n => n.innerHTMLString()).join(",");
  const cells = () => $$("span.c").map(n => n.innerHTMLString()).join(",");
  await click(".go");
  equal(gids(), "2,1");
  equal(cells(), "3,4,1,2"); // inner content travels with the moved outer item
};

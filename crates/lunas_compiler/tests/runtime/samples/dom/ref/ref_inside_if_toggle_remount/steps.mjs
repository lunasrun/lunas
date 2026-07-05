export default async ({ $$, click, equal }) => {
  const [toggleBtn, markBtn] = $$("button");
  await click(toggleBtn); // hide
  await click(toggleBtn); // show again (fresh node)
  await click(markBtn);
  const p = $$("p")[0];
  equal(p.getAttribute("data-marked"), "yes");
};

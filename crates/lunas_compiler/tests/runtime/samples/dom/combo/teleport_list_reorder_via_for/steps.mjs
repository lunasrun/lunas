export default async ({ click, equal }) => {
  const labels = () => {
    const ul = document.body.querySelector(".ported-teleport-list");
    return ul.childNodes
      .filter((n) => n.kind === "element")
      .map((n) => n.innerHTMLString())
      .join(",");
  };
  equal(labels(), "a,b,c");
  await click("button");
  equal(labels(), "c,b,a");
};

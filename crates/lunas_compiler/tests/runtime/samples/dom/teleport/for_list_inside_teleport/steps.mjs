export default async ({ equal }) => {
  const ul = document.body.querySelector(".ported-for-list");
  if (!ul) throw new Error("expected teleported list");
  const items = ul.childNodes
    .filter((n) => n.kind === "element")
    .map((li) => li.innerHTMLString());
  equal(items.join(","), "x,y,z");
};

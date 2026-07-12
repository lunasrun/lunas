export default async ({ equal }) => {
  const el = document.body.querySelector(".ported-html-binding");
  if (!el) throw new Error("expected teleported node");
  equal(el.innerHTMLString(), "<b>bold ported</b>");
};

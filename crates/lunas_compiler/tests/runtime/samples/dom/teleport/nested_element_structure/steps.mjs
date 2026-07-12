export default async ({ equal }) => {
  const el = document.body.querySelector(".ported-nested-structure");
  if (!el) throw new Error("expected teleported node");
  equal(el.innerHTMLString(), "<header>Title</header><section><p>body text</p></section>");
};

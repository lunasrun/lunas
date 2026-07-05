export default async ({ equal }) => {
  const a = document.body.querySelector(".ported-multi-a");
  const b = document.body.querySelector(".ported-multi-b");
  if (!a || !b) throw new Error("expected both teleported nodes");
  equal(a.innerHTMLString(), "first");
  equal(b.innerHTMLString(), "second");
};

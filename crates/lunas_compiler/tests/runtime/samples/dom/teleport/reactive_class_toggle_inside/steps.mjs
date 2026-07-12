export default async ({ click, equal }) => {
  const cls = () =>
    document.body.querySelector(".ported-reactive-class-toggle").getAttribute("class");
  equal(cls(), "ported-reactive-class-toggle off");
  await click("button");
  equal(cls(), "ported-reactive-class-toggle on");
};

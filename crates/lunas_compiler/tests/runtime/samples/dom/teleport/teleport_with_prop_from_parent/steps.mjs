export default async ({ click, equal }) => {
  const text = () => document.body.querySelector(".ported-prop-from-parent").innerHTMLString();
  equal(text(), "Hello");
  await click("button");
  equal(text(), "Renamed");
};

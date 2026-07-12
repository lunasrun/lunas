export default async ({ click, equal }) => {
  const text = () => document.body.querySelector(".ported-inside-child").innerHTMLString();
  equal(text(), "0");
  await click("button");
  equal(text(), "1");
};

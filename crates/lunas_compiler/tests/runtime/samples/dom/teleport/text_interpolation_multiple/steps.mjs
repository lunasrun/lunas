export default async ({ click, equal }) => {
  const text = () => document.body.querySelector(".ported-text-multi").innerHTMLString();
  equal(text(), "hi, sam!");
  await click("button");
  equal(text(), "hi, alex!");
};

export default async ({ click, equal }) => {
  const text = () => document.body.querySelector(".ported-toast-combo").innerHTMLString();
  equal(text(), "saved!");
  await click("button");
  equal(text(), "saved again!");
};

export default async ({ click, equal }) => {
  const count = () =>
    document.body.querySelector(".count-content-with-dynamics").innerHTMLString();
  equal(count(), "0");
  await click("button");
  equal(count(), "1");
  await click("button");
  equal(count(), "2");
};

// A `:for` over PRIMITIVE items (numbers) that mounts a child component per
// item used to crash (mountChild wrote `_children` onto the number). Assert the
// initial render, then push (mount a new child) and remove (tear a child down).
export default async ({ $$, click, expect, equal }) => {
  expect("b").count(3);
  expect($$("b")[0]).text("x=1");
  expect($$("b")[2]).text("x=3");

  // push 4 -> a fourth child mounts
  await click($$("button")[0]);
  expect("b").count(4);
  expect($$("b")[3]).text("x=4");

  // remove the middle item (2) -> its child tears down, order preserved
  await click($$("button")[1]);
  expect("b").count(3);
  equal(
    $$("b").map((n) => n.innerHTMLString()).join(","),
    "x=1,x=3,x=4"
  );
};

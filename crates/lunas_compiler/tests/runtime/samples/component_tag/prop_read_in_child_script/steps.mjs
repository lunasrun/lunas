export default async ({ $$, click, expect }) => {
  // The child's handler reads its prop `step`; it must unwrap to the value,
  // not the wrapped box object ("[object Object]").
  const childBtn = $$("button")[1];
  await click(childBtn);
  expect(childBtn).text("step is 2");
  // Parent pushes a new prop value; the child handler reads the fresh value.
  await click($$("button")[0]);
  await click(childBtn);
  expect(childBtn).text("step is 5");
};

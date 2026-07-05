export default async ({ $$, dispatch, expect }) => {
  const [a, b, c] = $$("input");
  expect("p").text("false false false");
  b.checked = true;
  await dispatch(b, "change");
  expect("p").text("false true false");
};

export default async ({ $$, dispatch, expect }) => {
  const [a, b] = $$("input");
  expect("span").text("a=false b=false");
  a.checked = true;
  await dispatch(a, "change");
  expect("span").text("a=true b=false");
  b.checked = true;
  await dispatch(b, "change");
  expect("span").text("a=true b=true");
};

export default async ({ $, dispatch, expect }) => {
  expect("input").attr("class", "chk");
  const input = $("input");
  input.checked = true;
  await dispatch(input, "change");
  expect("input").attr("class", "chk checked");
};

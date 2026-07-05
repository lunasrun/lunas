export default async ({ $, dispatch, expect }) => {
  expect("span").text("not agreed");
  const input = $("input");
  input.checked = true;
  await dispatch(input, "change");
  expect("span").text("agreed");
};

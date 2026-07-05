export default async ({ $, dispatch, expect }) => {
  expect("span").text("false");
  const input = $("input");
  input.checked = true;
  await dispatch(input, "change");
  expect("span").text("true");
};
